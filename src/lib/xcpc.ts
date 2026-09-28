export type XcpcTier = 'gold' | 'silver' | 'bronze' | 'iron';

export interface XcpcProblem {
  index: string;
  name: string;
  url: string;
  problemId: string;
  tier: XcpcTier | null;
  acceptedTeams: number | null;
  totalTeams: number | null;
  tagAxes?: string[];
  tagWeights?: Record<string, number>;
  tags?: string[];
  solved: boolean;
}

export interface XcpcContest {
  id: string;
  name: string;
  shortName: string;
  url: string;
  date: string;
  year: string;
  series: Array<'ICPC' | 'CCPC' | '省赛' | '其他'>;
  stage: string;
  site: string;
  boardSource: string | null;
  ratingsStale: boolean;
  problems: XcpcProblem[];
}

// A provincial contest can be both a series and a stage. Deduplicate only the
// displayed labels; keep both fields intact for their independent filters.
export function contestTags(contest: XcpcContest) {
  const tags = [
    ...contest.series.map((label) => ({ label, className: 'series' })),
    { label: contest.stage, className: '' },
    { label: contest.site, className: '' },
    ...(contest.boardSource ? [{ label: contest.boardSource, className: 'board' }] : []),
  ];
  const seen = new Set<string>();
  return tags.filter(({ label }) => {
    if (!label || seen.has(label)) return false;
    seen.add(label);
    return true;
  });
}
