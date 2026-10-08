<p align="center"><img src="src/assets/branding/oji-logo.png" alt="OJ Insight logo" width="144" /></p>

# OJ Insight

[![Read in Chinese](docs/assets/read-in-chinese.svg)](README.zh-CN.md)

**A local home for competitive programming progress, practice, and contest review.**

OJ Insight brings records from Codeforces, AtCoder, Luogu, NowCoder, QOJ, and LeetCode into one desktop app. It also includes an ICPC / CCPC Tracker. Instead of flattening every judge into a single score, it keeps each platform's rating and difficulty scale and shows which data is verified, aggregated, or unavailable.

[Download the latest release](https://github.com/Whalica/OJ_Insight/releases/latest) · [User manual](docs/manual/user-manual.pdf) · [Release notes](docs/UPDATEINFO.md) · [Report an issue](https://github.com/Whalica/OJ_Insight/issues)

Current version: **v0.10.5** · Windows, macOS, and Linux.

## See your progress across judges

The dashboard brings together solved problems, accepted submissions, active days, streaks, rating history, recent ACs, and difficulty distribution. Activity heatmaps cover a calendar year or the most recent 365 days, with First AC, Unique AC, AC Submissions, and the judge's own Activity metric kept distinct. Charts can be exported as PNG or SVG.

Multiple accounts on one judge can be tracked. Each judge retains its own rating and difficulty scale; OJ Insight does not invent problem-level submissions from a daily total or turn incompatible scales into a cross-platform score. When a source is unavailable, the app shows its status and keeps previously cached data visible.

## Build a training routine

- **Problem Sets** collect problems across judges, with roles, notes, tags, import and export. A set can become an independent mock contest.
- **Community Sets** present reviewed lists from [OJ Insight Community](https://github.com/Whalica/OJ_Insight-Community), including nested folders. They can be previewed against local completion records and saved as local copies. Personal submissions, notes, and code are never uploaded automatically.
- **Favorites** organizes links to problems, sets, articles, and other resources. Categories, search, pinning, and editable titles, summaries, and Markdown notes help keep references findable without copying the original pages.
- **Study Assistant** times a practice session and captures Markdown notes and mistakes. Finished sessions go to the **Solve Journal**; unfinished work remains a draft. The journal supports search, filters, and JSON / CSV export. Competitive Companion can prepare a draft for the current problem while the desktop app is running.
- **Personalized Training** draws candidates from reliable Codeforces, AtCoder, and QOJ catalogs while excluding locally solved problems. It exports a self-contained ZIP with a profile, constraints, candidates, and instructions for a language model; the resulting contest JSON can be imported as a set or contest.

## Practice and review contests

Mock contests keep their own configurations and session histories. Virtual contests support a pre-contest countdown, pause and resume, problem notes, and a whole-contest review. Solutions are still submitted to the original judge; OJ Insight reflects local AC records and available Codeforces / AtCoder verdicts without guessing missing wrong answers.

**Contest Review** packages finished virtual contests or official AtCoder contests for deeper analysis. Packages can include the problem list, submission timeline, Markdown notes, available code, and a starting document for a language model.

The **ICPC / CCPC Tracker** brings together ICPC, CCPC, and provincial contest problem sets. It shows upsolving progress from local QOJ records and, where public standings are available, problem tiers. Filters cover year, stage, site, series, completion, and tier. **Following** watches public accounts of friends or teammates and highlights new ACs.

## Supported judges

| Judge | Available information |
|---|---|
| Codeforces | Problem-level ACs, difficulty, and rating |
| AtCoder | Problem-level ACs, difficulty, and Algorithm Rating |
| Luogu | Submissions or public activity, solved counts, and official difficulty when available |
| NowCoder | Regular-problem ACs and Tracker completion |
| QOJ | Problem-level ACs and ICPC / CCPC upsolving; full submission history requires login |
| LeetCode | Solved counts and difficulty; international-site contest rating where available |

The upstream sites control what is public and may change their interfaces. OJ Insight marks missing or restricted information instead of presenting it as zero or as a complete record.

## Local by design

Accounts, synced records, training history, and exports stay in the app's local data directory; no OJ Insight account is needed. Standard personal-data exports omit login credentials. The app can back up its database and check for signed updates.

The English interface is **in development**. Navigation and some training tools have English text; other screens remain in Chinese. The language selector is temporarily hidden. Problem titles, judge tags, and personal notes keep their original language.

For setup, account credentials, backup, and troubleshooting, see the [user manual](docs/manual/user-manual.pdf). Technical architecture and source builds are documented separately in [Architecture](docs/ARCHITECTURE.md) and [Building](docs/BUILDING.md).
