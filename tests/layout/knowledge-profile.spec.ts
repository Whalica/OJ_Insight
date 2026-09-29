import { expect, test } from '@playwright/test';
import { buildKnowledgeProfile } from '../../src/lib/knowledge';

test('QOJ math profile needs repeated strong evidence and respects secondary tag weight', () => {
  const iron = Array.from({ length: 12 }, (_, index) => ({ problemId: `iron-${index}`, solved: true, tier: 'iron', tagWeights: { '图论与树': 1 } }));
  const gold = (index: number, mathWeight = 1) => ({ problemId: `gold-${index}`, solved: true, tier: 'gold', tagWeights: { '数学': mathWeight, '图论与树': 1 - mathWeight } });
  const one = buildKnowledgeProfile('qoj', [...iron, gold(0)]);
  const repeated = buildKnowledgeProfile('qoj', [...iron, ...Array.from({ length: 8 }, (_, index) => gold(index))]);
  const secondary = buildKnowledgeProfile('qoj', [...iron, gold(0, 0.1)]);
  const math = (profile: typeof one) => profile.find((bucket) => bucket.axis === '数学')!;
  expect(math(one).count).toBe(1);
  expect(math(one).score).toBeLessThan(60);
  expect(math(repeated).score).toBeGreaterThan(math(one).score);
  expect(math(secondary).score).toBeLessThan(math(one).score);
  expect(math(buildKnowledgeProfile('qoj', [...iron, gold(0), gold(0)])).count).toBe(1);
});
