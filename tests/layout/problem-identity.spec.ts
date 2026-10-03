import { expect, test } from '@playwright/test';
import { parseProblemUrl } from '../../src/lib/problemIdentity';

const uuid = '2dfbe5ea4d7c4b5db336f1f049ccf7cc';

for (const [url, problemKey] of [
  ['https://ac.nowcoder.com/acm/contest/78807/D', '78807/D'],
  ['https://ac.nowcoder.com/acm/contest/78807/D1/', '78807/D1'],
  ['https://ac.nowcoder.com/acm/problem/269161', '269161'],
  [`https://www.nowcoder.com/practice/${uuid}`, uuid],
  [`https://nowcoder.com/questionTerminal/${uuid}`, uuid],
]) {
  test(`NowCoder URL uses the sync problem key: ${url}`, () => {
    expect(parseProblemUrl(url)).toMatchObject({ platform: 'nowcoder', problemKey, problemId: problemKey, url });
  });
}

test('NowCoder identity ignores query and fragment while preserving the entered URL', () => {
  const url = 'https://ac.nowcoder.com/acm/contest/78807/D/?from=acm#submit';
  expect(parseProblemUrl(`  ${url}  `)).toMatchObject({ platform: 'nowcoder', problemKey: '78807/D', url });
});

test('lookalike NowCoder hosts are not assigned NowCoder identities', () => {
  for (const host of ['evilnowcoder.com', 'ac.nowcoder.com.example.org']) {
    for (const path of ['/acm/contest/78807/D', '/acm/problem/269161', `/practice/${uuid}`]) {
      expect(parseProblemUrl(`https://${host}${path}`)?.platform).toBe('other');
    }
  }
});

test('non-problem ACM paths do not become NowCoder problem identities', () => {
  for (const path of ['/acm/contest/78807', '/acm/contest/profile/78807', '/acm/problem/269161/editorial', '/acm/contest/78807/D/editorial']) {
    expect(parseProblemUrl(`https://ac.nowcoder.com${path}`)?.platform).toBe('other');
  }
});
