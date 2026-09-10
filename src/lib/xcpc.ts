export type XcpcTier = 'gold' | 'silver' | 'bronze' | 'iron';

export interface XcpcProblem {
  index: string;
  name: string;
  url: string;
  problemId: string;
  tier: XcpcTier | null;
  acceptedTeams: number | null;
  totalTeams: number | null;
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
  problems: XcpcProblem[];
}

export function problemColumns(contests: XcpcContest[]): string[] {
  const indexes = new Set(contests.flatMap((contest) => contest.problems.map((problem) => problem.index)));
  return [...indexes].sort((a, b) => a.localeCompare(b, 'en'));
}
