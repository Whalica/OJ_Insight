import codeforcesIcon from '../assets/platforms/codeforces.ico';
import atcoderIcon from '../assets/platforms/atcoder.png';
import luoguIcon from '../assets/platforms/luogu.ico';
import nowcoderIcon from '../assets/platforms/nowcoder.ico';
import leetcodeIcon from '../assets/platforms/leetcode.ico';
import { PLATFORM_META } from '../lib/platforms';
import type { TrainingPlatform } from '../types';

const PLATFORM_ICONS: Partial<Record<TrainingPlatform, string>> = {
  codeforces: codeforcesIcon,
  atcoder: atcoderIcon,
  luogu: luoguIcon,
  nowcoder: nowcoderIcon,
  leetcode: leetcodeIcon,
};

interface Props {
  platform: TrainingPlatform;
  className?: string;
}

export default function PlatformIcon({ platform, className = '' }: Props) {
  const meta = PLATFORM_META[platform];
  const icon = PLATFORM_ICONS[platform];

  return <span className={`platform-icon ${className}`.trim()} aria-hidden="true">
    {icon
      ? <img src={icon} alt="" />
      : <span className="platform-icon-fallback" style={{ color: meta.accent }}>{meta.short}</span>}
  </span>;
}
