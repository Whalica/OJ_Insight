export type XcpcTier = 'gold' | 'silver' | 'bronze' | 'iron';

export interface XcpcProblem {
  index: string;
  name: string;
  tier: XcpcTier;
  acceptedTeams: number;
  totalTeams: number;
  solved: boolean;
}

export interface XcpcContest {
  id: string;
  name: string;
  shortName: string;
  date: string;
  year: string;
  series: Array<'ICPC' | 'CCPC' | '省赛' | '其他'>;
  stage: '区域赛' | '分站赛' | '网络赛' | '邀请赛' | '省赛';
  site: string;
  boardSource: 'RankLand' | 'XCPCIO';
  problems: XcpcProblem[];
}

export const XCPC_MAX_PROBLEMS = 13;

const problemNames = [
  'Bridge Reconstruction', 'Binary Tree', 'Circular Paths', 'Decomposition',
  'Expected Value', 'Flow Control', 'Grid Walk', 'Hidden String',
  'Interval Query', 'Journey', 'Kingdom', 'Linear Basis', 'Mountain',
];

const tierTemplates: XcpcTier[][] = [
  ['iron', 'silver', 'gold', 'iron', 'bronze', 'gold', 'silver', 'iron', 'bronze', 'silver', 'iron', 'gold', 'gold'],
  ['iron', 'bronze', 'silver', 'iron', 'gold', 'silver', 'bronze', 'gold', 'iron', 'silver', 'gold', 'bronze', 'iron'],
  ['silver', 'iron', 'gold', 'bronze', 'silver', 'gold', 'iron', 'bronze', 'iron', 'gold', 'silver', 'gold', 'iron'],
];

function makeProblems(total: number, solved: number, template: number): XcpcProblem[] {
  const accepted = [276, 184, 121, 73, 48, 31, 19, 12, 8, 5, 3, 1, 0];
  return Array.from({ length: total }, (_, index) => ({
    index: String.fromCharCode(65 + index),
    name: problemNames[index],
    tier: tierTemplates[template % tierTemplates.length][index],
    acceptedTeams: Math.max(0, accepted[index] - template * 3),
    totalTeams: 312 - template * 4,
    solved: index < solved,
  }));
}

// Initial catalog adapter. It is intentionally isolated so a QOJ/board-backed loader can
// replace the seed without changing the page or the existing platform data contracts.
export const XCPC_CONTESTS: XcpcContest[] = [
  { id: 'icpc-2025-shanghai', name: '2025 ICPC Asia Shanghai Regional Contest', shortName: '2025 ICPC 上海站', date: '2025-11-23', year: '2025', series: ['ICPC'], stage: '区域赛', site: '上海', boardSource: 'XCPCIO', problems: makeProblems(13, 9, 0) },
  { id: 'ccpc-2025-harbin', name: '第十一届中国大学生程序设计竞赛 哈尔滨站', shortName: '2025 CCPC 哈尔滨站', date: '2025-11-09', year: '2025', series: ['CCPC'], stage: '分站赛', site: '哈尔滨', boardSource: 'RankLand', problems: makeProblems(12, 12, 1) },
  { id: 'icpc-2025-nanjing', name: '2025 ICPC Asia Nanjing Regional Contest', shortName: '2025 ICPC 南京站', date: '2025-11-02', year: '2025', series: ['ICPC'], stage: '区域赛', site: '南京', boardSource: 'XCPCIO', problems: makeProblems(13, 7, 2) },
  { id: 'icpc-2025-online-2', name: '2025 ICPC Asia EC 网络预选赛（第二场）', shortName: '2025 ICPC 网络赛 2', date: '2025-09-14', year: '2025', series: ['ICPC'], stage: '网络赛', site: '全国', boardSource: 'RankLand', problems: makeProblems(12, 6, 0) },
  { id: 'ccpc-2025-nanchang', name: '2025 CCPC 全国邀请赛（南昌）暨江西省赛', shortName: '2025 CCPC 南昌邀请赛', date: '2025-05-18', year: '2025', series: ['CCPC', '省赛'], stage: '邀请赛', site: '南昌', boardSource: 'RankLand', problems: makeProblems(13, 8, 1) },
  { id: 'ccpc-2024-chengdu', name: '2024 中国大学生程序设计竞赛 成都站', shortName: '2024 CCPC 成都站', date: '2024-11-17', year: '2024', series: ['CCPC'], stage: '分站赛', site: '成都', boardSource: 'XCPCIO', problems: makeProblems(13, 4, 2) },
  { id: 'icpc-2024-kunming', name: '2024 ICPC Kunming Invitational Contest', shortName: '2024 ICPC 昆明邀请赛', date: '2024-04-21', year: '2024', series: ['ICPC'], stage: '邀请赛', site: '昆明', boardSource: 'RankLand', problems: makeProblems(13, 5, 0) },
  { id: 'shanghai-2023', name: '2023 上海市大学生程序设计竞赛', shortName: '2023 上海市赛', date: '2023-05-07', year: '2023', series: ['省赛'], stage: '省赛', site: '上海', boardSource: 'RankLand', problems: makeProblems(11, 5, 1) },
];
