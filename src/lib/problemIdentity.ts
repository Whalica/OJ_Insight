import type { CanonicalProblem, Platform } from '../types';

type ParsedIdentity = Pick<CanonicalProblem, 'platform' | 'problemKey' | 'problemId' | 'name' | 'url'>;

export function parseProblemUrl(value: string): ParsedIdentity | null {
  const input = value.trim();
  if (!input) return null;
  let url: URL;
  try { url = new URL(input); } catch { return null; }
  const host = url.hostname.replace(/^www\./, '').toLowerCase();
  const path = url.pathname.replace(/\/+$/, '');

  let match = path.match(/^\/(?:contest|gym)\/(\d+)\/problem\/([^/]+)$/i) || path.match(/^\/problemset\/problem\/(\d+)\/([^/]+)$/i);
  if (host === 'codeforces.com' && match) return identity('codeforces', `${match[1]}:${match[2]}`, `${match[1]}${match[2]}`, input);

  match = path.match(/^\/contests\/([^/]+)\/tasks\/([^/]+)$/i);
  if (host === 'atcoder.jp' && match) return identity('atcoder', match[2], match[2], input);

  match = path.match(/^\/problem\/([A-Z][A-Z0-9_]*)$/i);
  if (host === 'luogu.com.cn' && match) return identity('luogu', match[1].toUpperCase(), match[1].toUpperCase(), input);

  match = path.match(/^\/problem\/(\d+)$/i);
  if (host === 'qoj.ac' && match) return identity('qoj', match[1], match[1], input);

  match = path.match(/^\/problems\/([^/]+)$/i);
  if ((host === 'leetcode.com' || host === 'leetcode.cn') && match) return identity('leetcode', match[1], match[1], input);

  match = path.match(/\/(?:questionTerminal|practice)\/([^/?#]+)/i);
  if (host.endsWith('nowcoder.com') && match) return identity('nowcoder', match[1], match[1], input);
  return null;
}

function identity(platform: Platform, problemKey: string, problemId: string, url: string): ParsedIdentity {
  const short: Record<Platform, string> = { codeforces: 'CF', atcoder: 'AtCoder', luogu: '洛谷', nowcoder: '牛客', qoj: 'QOJ', leetcode: 'LeetCode' };
  return { platform, problemKey, problemId, name: `${short[platform]} ${problemId}`, url };
}
