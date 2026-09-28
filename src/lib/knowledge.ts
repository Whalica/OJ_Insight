import type { KnowledgeBucket, Platform } from '../types';

export const KNOWLEDGE_AXES = ['基础与模拟', '数据结构', '图论与树', '动态规划', '数学', '字符串', '搜索与构造', '贪心与思维'] as const;
type KnowledgeAxis = typeof KNOWLEDGE_AXES[number];

export function knowledgeAxis(value: string): KnowledgeAxis | null {
  const tag = value.trim().toLowerCase();
  if (!tag) return null;
  if (['data structures', 'data structure', 'array', 'hash', 'stack', 'queue', 'heap', 'linked list', 'segment tree', 'fenwick', 'dsu', '数据结构'].some((item) => tag.includes(item))) return '数据结构';
  if (['graph', 'tree', 'shortest path', 'mst', 'topological', '图论', '树'].some((item) => tag.includes(item))) return '图论与树';
  if (['dynamic programming', 'dp', '动态规划'].some((item) => tag === item || tag.includes(item))) return '动态规划';
  if (['math', 'number theory', 'combinatorics', 'geometry', 'probability', '数学', '几何'].some((item) => tag.includes(item))) return '数学';
  if (['string', 'trie', '字符串'].some((item) => tag.includes(item))) return '字符串';
  if (['binary search', 'brute force', 'backtracking', 'dfs', 'bfs', 'constructive', 'search', '搜索', '构造'].some((item) => tag.includes(item))) return '搜索与构造';
  if (['greedy', 'two pointers', 'sliding window', 'divide and conquer', 'sort', '贪心', '思维'].some((item) => tag.includes(item))) return '贪心与思维';
  if (['implementation', 'simulation', 'basic', '基础', '模拟', '算法策略'].some((item) => tag.includes(item))) return '基础与模拟';
  return null;
}

function weightedQuantile(items: Array<{ value: number; weight: number }>, quantile = .75) {
  const sorted = items.filter((item) => item.weight > 0).sort((left, right) => left.value - right.value);
  const target = sorted.reduce((sum, item) => sum + item.weight, 0) * quantile;
  let seen = 0;
  for (const item of sorted) { seen += item.weight; if (seen >= target) return item.value; }
  return sorted.at(-1)?.value ?? 0;
}

export function knowledgeDifficultyLevel(platform: Platform, difficulty?: string | null, tier?: string | null) {
  const label = (tier || difficulty || '').trim().toLowerCase();
  if (platform === 'codeforces') {
    const rating = Number(label);
    return Number.isFinite(rating) && rating > 0 ? Math.max(5, Math.min(95, 20 + .05 * (rating - 800))) : null;
  }
  if (platform === 'leetcode') return label === 'hard' ? 86 : label === 'medium' ? 65 : label === 'easy' ? 42 : null;
  if (platform === 'qoj') return label.includes('gold') || label.includes('金') ? 90 : label.includes('silver') || label.includes('银') ? 76 : label.includes('bronze') || label.includes('铜') ? 58 : label.includes('iron') || label.includes('铁') ? 38 : null;
  return null;
}

export function buildKnowledgeProfile(platform: Platform, problems: Array<{ problemId?: string; solved?: boolean; tagAxes?: string[]; tags?: string[]; difficulty?: string | null; tier?: string | null }>): KnowledgeBucket[] {
  const evidence = new Map<string, Array<{ value: number; weight: number }>>();
  const all: Array<{ value: number; weight: number }> = [];
  const seenProblems = new Set<string>();
  for (const problem of problems.filter((item) => item.solved !== false)) {
    const axes = new Set([...(problem.tagAxes || []), ...(problem.tags || [])].map(knowledgeAxis).filter((axis): axis is KnowledgeAxis => axis !== null));
    const value = knowledgeDifficultyLevel(platform, problem.difficulty, problem.tier);
    if (value == null || !axes.size) continue;
    if (problem.problemId) {
      if (seenProblems.has(problem.problemId)) continue;
      seenProblems.add(problem.problemId);
    }
    const item = { value, weight: 1 / axes.size };
    all.push(item);
    for (const axis of axes) evidence.set(axis, [...(evidence.get(axis) || []), item]);
  }
  if (![...evidence.values()].some((items) => items.length > 0)) return [];
  const prior = weightedQuantile(all) || 50;
  return KNOWLEDGE_AXES.map((axis) => {
    const items = evidence.get(axis) || [];
    if (!items.length) return { platform, axis, count: 0, score: 0 };
    const representative = weightedQuantile(items);
    const effective = Math.min(20, items.reduce((sum, item) => sum + item.weight, 0));
    if (platform === 'qoj') {
      const relative = 50 + (representative - prior) * 1.15;
      const estimate = representative * .58 + relative * .42;
      const confidence = effective / (effective + 4);
      return { platform, axis, count: items.length, score: Math.round(Math.max(8, Math.min(95, confidence * estimate + (1 - confidence) * 50))) };
    }
    const shrink = effective / (effective + 6);
    return { platform, axis, count: items.length, score: Math.round(Math.max(5, Math.min(95, shrink * representative + (1 - shrink) * prior))) };
  });
}

export function mergeKnowledgeBuckets(data: KnowledgeBucket[], platform?: Platform | null) {
  return KNOWLEDGE_AXES.map((axis) => {
    const items = data.filter((item) => item.axis === axis && (!platform || item.platform === platform));
    const count = items.reduce((sum, item) => sum + item.count, 0);
    const confidence = items.reduce((sum, item) => sum + Math.sqrt(item.count), 0);
    const score = confidence ? Math.round(items.reduce((sum, item) => sum + item.score * Math.sqrt(item.count), 0) / confidence) : 0;
    return { platform: platform || 'codeforces' as Platform, axis, count, score };
  });
}
