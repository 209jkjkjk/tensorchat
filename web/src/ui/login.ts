/**
 * The sign-in / sign-up screen.
 *
 * Deliberately one screen with a mode toggle rather than two routes: it is the
 * only pre-authentication surface in the app, and keeping it in one place means
 * the session-establishing code path exists exactly once.
 */

import { el, replace } from '../dom.ts';
import { siteName } from '../brand.ts';
import { ApiError, api, setToken } from '../api.ts';
import type { User } from '../protocol.ts';

/**
 * Pull an invite token out of `#/join/{token}`, if the URL carries one.
 *
 * The fragment rather than a query string, deliberately: fragments are not sent
 * to the server on a page load and do not end up in access logs, which matters
 * for a credential that is live until it is spent.
 */
export function inviteFromLocation(hash: string): string | null {
  const m = /^#\/join\/([^/?#]+)$/.exec(hash);
  return m ? decodeURIComponent(m[1]!) : null;
}

export function LoginScreen(onAuthenticated: (user: User) => void): HTMLElement {
  const invite = inviteFromLocation(location.hash);
  // An invite link is an instruction to create an account, so open on the
  // sign-up form rather than making the recipient find the toggle.
  let mode: 'login' | 'register' = invite ? 'register' : 'login';
  let busy = false;
  // null while the check is in flight, so the form does not flash a rejection
  // before the server has answered.
  let inviteValid: boolean | null = invite ? null : false;

  const error = el('div', { class: 'auth-error', hidden: true, role: 'alert' });
  const form = el('form', { class: 'auth-form' });
  const root = el(
    'div',
    { class: 'auth-screen' },
    el(
      'div',
      { class: 'auth-card' },
      el('h1', { class: 'auth-title', text: siteName() }),
      el('p', { class: 'auth-sub', text: '快速、自托管的团队聊天。' }),
      form,
    ),
  );

  const handle = el('input', {
    class: 'auth-input',
    type: 'text',
    placeholder: '用户名',
    autocomplete: 'username',
    required: 'required',
  }) as HTMLInputElement;

  const displayName = el('input', {
    class: 'auth-input',
    type: 'text',
    placeholder: '显示名称',
    autocomplete: 'name',
  }) as HTMLInputElement;

  const password = el('input', {
    class: 'auth-input',
    type: 'password',
    placeholder: '密码',
    autocomplete: 'current-password',
    required: 'required',
  }) as HTMLInputElement;

  const submit = el('button', {
    class: 'auth-submit',
    type: 'submit',
    text: '登录',
  }) as HTMLButtonElement;

  const toggle = el('button', {
    class: 'auth-toggle',
    type: 'button',
    text: '创建账号',
    on: {
      click: () => {
        mode = mode === 'login' ? 'register' : 'login';
        render();
      },
    },
  });

  const inviteNote = el('p', { class: 'auth-invite' });

  /**
   * The "Sign in with …" button, once the server has said there is a provider.
   *
   * Built lazily rather than hidden, so a server without one renders exactly
   * the form it rendered before this existed.
   */
  let providerButton: HTMLElement | null = null;

  function render(): void {
    const registering = mode === 'register';
    password.autocomplete = registering ? 'new-password' : 'current-password';
    submit.textContent = registering ? '创建账号' : '登录';
    toggle.textContent = registering ? '我已有账号' : '创建账号';

    // The banner only belongs on the sign-up form: an invite says nothing about
    // signing in to an account you already have.
    const showInvite = invite !== null && registering;
    if (showInvite) {
      inviteNote.textContent =
        inviteValid === null
          ? '正在验证邀请链接…'
          : inviteValid
            ? '你已获邀，请设置用户名后加入。'
            : '该邀请链接已过期或已被使用。';
      inviteNote.classList.toggle('is-dead', inviteValid === false);
    }
    // Nothing to submit against a dead link, and disabling says so before the
    // person types out a password.
    submit.disabled = busy || (showInvite && inviteValid === false);

    replace(form, [
      showInvite ? inviteNote : null,
      handle,
      registering ? displayName : null,
      password,
      error,
      submit,
      toggle,
      providerButton,
    ]);
    if (!submit.disabled) handle.focus();
  }

  // Fire and forget, like the invite check above: if this never answers, the
  // password form is still perfectly usable.
  void api
    .authProviders()
    .then((p) => {
      if (!p.oidc) return;
      providerButton = el(
        'div',
        { class: 'auth-provider' },
        el('div', { class: 'auth-or', text: '或' }),
        el('button', {
          class: 'auth-provider-button',
          type: 'button',
          text: `使用 ${p.oidc.label} 登录`,
          on: {
            // A full navigation, not a fetch. The provider answers with a
            // redirect to its own login page, which an XHR cannot follow —
            // and the CSP would not allow reaching it if it could.
            click: () => location.assign('/api/oauth/start'),
          },
        }),
      );
      render();
    })
    .catch(() => {
      // No providers, or no answer. Either way there is no button to draw.
    });

  if (invite) {
    // Fire and forget: a failure here only downgrades the banner, and the
    // server re-checks under the write lock when the form is actually
    // submitted. This is a courtesy, not the enforcement.
    void api
      .checkInvite(invite)
      .then((r) => {
        inviteValid = r.valid;
      })
      .catch(() => {
        inviteValid = false;
      })
      .finally(render);
  }

  function showError(message: string): void {
    error.textContent = message;
    error.hidden = false;
  }

  form.addEventListener('submit', async (ev: Event) => {
    ev.preventDefault();
    if (busy) return;
    error.hidden = true;

    const h = handle.value.trim().toLowerCase().replace(/^@/, '');
    const p = password.value;
    if (!h || !p) {
      showError('请输入用户名和密码。');
      return;
    }

    busy = true;
    submit.disabled = true;
    submit.textContent = mode === 'register' ? '正在创建…' : '正在登录…';
    try {
      const session =
        mode === 'register'
          ? await api.register(h, displayName.value.trim() || h, p, invite ?? undefined)
          : await api.login(h, p);
      setToken(session.token);
      // Drop the token out of the address bar before the app boots, so a
      // spent invite is not left sitting in history or a shared screenshot.
      if (invite) history.replaceState(null, '', location.pathname + location.search);
      onAuthenticated(session.user);
    } catch (err) {
      showError(
        err instanceof ApiError
          ? err.status === 401
            ? '用户名或密码不正确。'
            : err.message
          : '无法连接服务器。',
      );
      // Never leave a password sitting in the DOM after a failure.
      password.value = '';
      password.focus();
    } finally {
      // `render` owns the disabled state now — it also has to account for a
      // dead invite, so setting it here as well would fight with that.
      busy = false;
      render();
    }
  });

  render();
  return root;
}
