import { useEffect, useMemo, useState, type CSSProperties } from 'react';
import { ChevronDown } from 'lucide-react';
import { PLATFORM_META, PLATFORM_ORDER } from '../lib/platforms';
import type { Platform, RatingSummary } from '../types';

type Period = '30' | '90' | 'all';

function ratingLabel(platform: Platform, rating: number) {
  if (platform === 'codeforces') {
    if (rating >= 3000) return 'Legendary Grandmaster';
    if (rating >= 2600) return 'International Grandmaster';
    if (rating >= 2400) return 'Grandmaster';
    if (rating >= 2300) return 'International Master';
    if (rating >= 2100) return 'Master';
    if (rating >= 1900) return 'Candidate Master';
    if (rating >= 1600) return 'Expert';
    if (rating >= 1400) return 'Specialist';
    if (rating >= 1200) return 'Pupil';
    return 'Newbie';
  }
  if (platform === 'atcoder') return 'AtCoder Rating';
  if (platform === 'leetcode') return 'Contest Rating';
  return 'Rating';
}

function ratingColor(platform: Platform, rating: number) {
  if (platform === 'codeforces') {
    if (rating >= 2400) return '#f15b64';
    if (rating >= 2100) return '#f0b44c';
    if (rating >= 1900) return '#c979e7';
    if (rating >= 1600) return '#5e9ee6';
    if (rating >= 1400) return '#48c7c0';
    if (rating >= 1200) return '#62c878';
    return '#9aa4ad';
  }
  return PLATFORM_META[platform].accent;
}

function dateLabel(epoch: number, timeZone: string) {
  return new Intl.DateTimeFormat('zh-CN', {
    timeZone,
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  }).format(new Date(epoch * 1000));
}

export default function RatingOverview({ ratings, timeZone, selectedPlatform }: { ratings: RatingSummary[]; timeZone: string; selectedPlatform?: Platform | null }) {
  const available = useMemo(() => new Set(ratings.map((item) => item.platform)), [ratings]);
  const [platform, setPlatform] = useState<Platform>(() => selectedPlatform || ratings[0]?.platform || 'codeforces');
  const [account, setAccount] = useState('');
  const [period, setPeriod] = useState<Period>('90');
  useEffect(() => { if (selectedPlatform) setPlatform(selectedPlatform); }, [selectedPlatform]);
  const accounts = ratings.filter((item) => item.platform === platform);

  useEffect(() => {
    if (!accounts.some((item) => item.account === account)) setAccount(accounts[0]?.account || '');
  }, [platform, ratings, account]);

  const summary = accounts.find((item) => item.account === account) || accounts[0];
  const shown = useMemo(() => {
    if (!summary) return [];
    if (period === 'all') return summary.history;
    const cutoff = Date.now() / 1000 - Number(period) * 86400;
    return summary.history.filter((point) => point.epoch_second >= cutoff);
  }, [summary, period]);

  const chart = useMemo(() => {
    if (!shown.length) return null;
    const width = 720; const height = 184; const pad = 14;
    const low = Math.min(...shown.map((point) => point.new_rating));
    const high = Math.max(...shown.map((point) => point.new_rating));
    const spread = Math.max(80, high - low);
    const floor = low - spread * .18; const ceiling = high + spread * .18;
    const points = shown.map((point, index) => {
      const x = shown.length === 1 ? width / 2 : index * width / (shown.length - 1);
      const y = height - pad - (point.new_rating - floor) / (ceiling - floor) * (height - pad * 2);
      return { ...point, x, y };
    });
    return {
      points,
      line: points.map((point) => `${point.x.toFixed(1)},${point.y.toFixed(1)}`).join(' '),
      area: `M 0 ${height} L ${points.map((point) => `${point.x.toFixed(1)} ${point.y.toFixed(1)}`).join(' L ')} L ${width} ${height} Z`,
    };
  }, [shown]);

  const color = summary ? ratingColor(platform, summary.current) : PLATFORM_META[platform].accent;
  return <>
    <div className="section-title rating-title"><small>RATING OVERVIEW · 独立于训练时间范围</small><h2>竞赛 Rating 总览</h2></div>
    <section className="panel rating-panel" style={{ '--rating-color': color } as CSSProperties}>
      <div className="rating-tabs">
        {(selectedPlatform ? [selectedPlatform] : PLATFORM_ORDER).map((item) => <button className={item === platform ? 'active' : ''} onClick={() => setPlatform(item)} key={item}>
          <span className="oj-dot" style={{ background: PLATFORM_META[item].accent }} />{PLATFORM_META[item].short}<small>{PLATFORM_META[item].name}</small>{available.has(item) && <i />}
        </button>)}
      </div>
      {!summary ? <div className="rating-empty"><strong>{PLATFORM_META[platform].name} 暂无 Rating 记录</strong><span>{['codeforces', 'atcoder', 'leetcode'].includes(platform) ? '请配置 ID 后同步；未参加 Rated 比赛或接口暂不可用时不会显示为 0。LeetCode 当前仅接入国际站。' : '该平台的 Rating 历史暂未接入，不显示 0 或估算值。'}</span></div> : <div className="rating-layout">
        <div className="rating-summary">
          <div className="rating-account"><span className="platform-monogram" style={{ color: PLATFORM_META[platform].accent }}>{PLATFORM_META[platform].short}</span>{accounts.length > 1 ? <div className="select-wrap"><select value={summary.account} onChange={(event) => setAccount(event.target.value)}>{accounts.map((item) => <option key={item.account}>{item.account}</option>)}</select><ChevronDown size={14} /></div> : <strong>{summary.account}</strong>}</div>
          <div className="rating-current"><small>当前 Rating</small><strong>{summary.current.toLocaleString()}</strong><span>{ratingLabel(platform, summary.current)}</span><small>{summary.stale ? '本次 Rating 未更新，显示缓存' : 'Rating 已同步'}{summary.last_updated ? ` · ${dateLabel(summary.last_updated, timeZone)}` : ''}</small></div>
          <div className="rating-facts">
            <div><small>历史最高</small><strong>{summary.maximum.toLocaleString()}</strong></div>
            <div><small>最近一场变化</small><strong className={summary.last_change < 0 ? 'negative' : 'positive'}>{platform === 'leetcode' && summary.contest_count === 1 ? '—' : `${summary.last_change > 0 ? '+' : ''}${summary.last_change}`}</strong></div>
            <div><small>Rated 场次</small><strong>{summary.contest_count}</strong></div>
            <div><small>最近 Rated 比赛</small><strong>{dateLabel(summary.last_contest_epoch, timeZone)}</strong></div>
          </div>
        </div>
        <div className="rating-chart-wrap">
          <div className="rating-chart-head"><strong>{period === 'all' ? '完整 Rating 历史' : `最近 ${period} 天变化`}</strong><div className="rating-periods">{(['30', '90', 'all'] as Period[]).map((item) => <button className={item === period ? 'active' : ''} onClick={() => setPeriod(item)} key={item}>{item === 'all' ? '全部' : `${item} 天`}</button>)}</div></div>
          {chart ? <div className="rating-chart"><svg viewBox="0 0 720 184" preserveAspectRatio="none" role="img" aria-label={`${PLATFORM_META[platform].name} Rating 曲线`}><line x1="0" y1="28" x2="720" y2="28"/><line x1="0" y1="92" x2="720" y2="92"/><line x1="0" y1="156" x2="720" y2="156"/>{chart.points.length > 1 && <path d={chart.area}/>}<polyline points={chart.line}/>{chart.points.map((point) => <circle key={`${point.epoch_second}-${point.new_rating}`} cx={point.x} cy={point.y} r="4"><title>{point.contest_name} · {point.new_rating} · {dateLabel(point.epoch_second, timeZone)}</title></circle>)}</svg><div><span>{dateLabel(shown[0].epoch_second, timeZone)}</span><span>比赛 Rating 更新事件</span><span>{dateLabel(shown[shown.length - 1].epoch_second, timeZone)}</span></div></div> : <div className="rating-range-empty">这个时间范围内没有 Rating 更新，切换到“全部”可查看完整历史。</div>}
        </div>
      </div>}
    </section>
  </>;
}
