/** The first-run view for a workspace with no channels yet. */

import { ICONS, el, icon } from '../dom.ts';
import { retentionLabel, siteName } from '../brand.ts';

export type EmptyWorkspaceActions = {
  browseChannels: () => void;
  createChannel: () => void;
};

/**
 * Give a user with no conversation a clear next step without opening a modal
 * by itself. The buttons still use the existing browse/create flows.
 */
export function EmptyWorkspace(actions: EmptyWorkspaceActions): HTMLElement {
  const retention = retentionLabel();

  return el(
    'section',
    { class: 'empty-workspace', hidden: true, aria: { label: '开始使用' } },
    el('div', { class: 'empty-workspace-icon' }, icon(ICONS.hash, 28)),
    el('h2', { class: 'empty-workspace-title', text: `欢迎来到 ${siteName()}` }),
    el('p', {
      class: 'empty-workspace-copy',
      text: '这里还没有频道，加入频道或创建频道开始聊天。',
    }),
    el(
      'div',
      { class: 'empty-workspace-actions' },
      el(
        'button',
        {
          class: 'empty-workspace-action empty-workspace-action-primary',
          on: { click: actions.browseChannels },
        },
        icon(ICONS.people, 20),
        el(
          'span',
          { class: 'empty-workspace-action-text' },
          el('span', { class: 'empty-workspace-action-label', text: '加入频道' }),
          el('span', { class: 'empty-workspace-action-hint', text: '浏览公开频道' }),
        ),
      ),
      el(
        'button',
        { class: 'empty-workspace-action', on: { click: actions.createChannel } },
        icon(ICONS.plus, 20),
        el(
          'span',
          { class: 'empty-workspace-action-text' },
          el('span', { class: 'empty-workspace-action-label', text: '创建频道' }),
          el('span', { class: 'empty-workspace-action-hint', text: '创建一个新的聊天空间' }),
        ),
      ),
    ),
    retention
      ? el('p', {
          class: 'empty-workspace-warning',
          text: `警告：超过${formatRetention(retention)}的聊天记录的会自动删除（服务器设置）`,
        })
      : el('p', {
          class: 'empty-workspace-warning',
          text: '警告：服务器未启用聊天记录自动删除',
        }),
  );
}

function formatRetention(value: string): string {
  const match = /^(\d+)([mhd])$/.exec(value);
  if (!match) return value;
  const units = { m: '分钟', h: '小时', d: '天' } as const;
  return `${match[1]}${units[match[2] as keyof typeof units]}`;
}
