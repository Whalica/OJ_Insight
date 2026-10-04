# OJ Insight

[![Read in Chinese](docs/assets/read-in-chinese.svg)](README.zh-CN.md)

**Turn practice records scattered across Online Judges into a clear, trustworthy history of your progress.**

OJ Insight is a local-first desktop dashboard for competitive programmers. It supports Codeforces, AtCoder, Luogu, NowCoder, QOJ, and LeetCode, and includes a separate ICPC / CCPC Tracker. It keeps your data on your device and distinguishes verified submissions, public aggregates, and unavailable data instead of presenting incompatible sources as one complete record.

[Download the latest release](https://github.com/Whalica/OJ_Insight/releases/latest) · [Release notes (Chinese)](docs/UPDATEINFO.md) · [Report an issue](https://github.com/Whalica/OJ_Insight/issues) · [Build from source (Chinese)](docs/BUILDING.md) · [User manual (Chinese)](docs/manual/user-manual.pdf)

Current version: **v0.10.3**. Available for Windows, macOS, and Linux.

The English interface is still in development. Navigation and several training tools have initial translations, while other screens remain in Chinese. The language selector is temporarily hidden. Problem titles, judge tags, and your notes stay in their original language.

## Why use OJ Insight?

### One dashboard for six judges

See your accounts, activity, solved counts, difficulty distribution, and rating history together. You can configure multiple accounts on the same platform.

### Broad coverage without inventing data

Judges expose different kinds of information. OJ Insight distinguishes:

- verifiable, problem-level accepted submissions;
- daily activity or solved-count aggregates supplied by a platform;
- information that cannot currently be retrieved.

The app does not fabricate submissions when only aggregates are available, or convert ratings and difficulty scales into a misleading cross-platform score. A failed sync updates the error status while leaving previously cached data available.

### ICPC / CCPC upsolving

The ICPC / CCPC Tracker collects ICPC, CCPC, and provincial contest problem sets. Filter by year, stage, site, series, completion progress, and problem tier; gold, silver, bronze, and iron tiers can be selected together. Completion comes from local QOJ records. When public standings are available, the tracker also shows the corresponding problem tiers.

### Problem sets, mock contests, virtual contests, and review

**Problem Sets** organize and share problems across judges. Paste a problem URL to identify its platform and problem ID offline, then fetch or enter a title and tags as needed. **Mock Contests** store independent configurations and past sessions. Create one from a problem set, build it manually, or import an AI-generated result. Contest descriptions support Markdown and LaTeX.

**Favorites** is a local link organizer for problems, problem sets, articles, and other resources. Add a web link, edit its displayed title, summary, and Markdown note, and organize it with categories, search, and pinning. It keeps a link to the original page rather than a copy of its content. Deleting a category moves its links to Uncategorized.

In the virtual contest area, you can save a pre-contest countdown. The countdown starts only when you select **Start VP**; the contest begins when it reaches zero. During a VP, you can pause, resume, or finish the session and record notes for individual problems or the whole contest. Submit solutions on the original judge. OJ Insight updates progress from local AC syncs and available Codeforces / AtCoder verdicts; it does not infer a wrong answer when failed submissions are unavailable.

The floating **Study Assistant** times a practice session, keeps Markdown notes, and records mistakes. Ending a session saves it to the **Solve Journal**; closing the assistant keeps a draft. The journal supports search, filters, and JSON / CSV exports. With the desktop app running, Competitive Companion can create a draft for the current problem; you start the timer when you are ready.

**Personalized Training** builds a candidate pool from reliable Codeforces, AtCoder, and QOJ catalogs rather than treating existing problem sets as its initial candidates. It excludes locally solved problems and exports a self-contained ZIP with your profile, constraints, candidates, and complete instructions. Upload the ZIP to a language model to generate contest JSON; extra requirements are optional. Import the result as a problem set or contest.

**Community Sets** browse reviewed lists from [OJ Insight Community](https://github.com/Whalica/OJ_Insight-Community). Preview a set, compare it with local completion records, and save a local copy. You can prepare a community submission JSON from a local set and submit it through a repository PR. Personal submissions, notes, and code are never uploaded automatically. Luogu lists can also be imported by URL; if login is required, you may supply a temporary Luogu Cookie for that import. Recognized Codeforces and AtCoder problems retain their original identities and links.

### Contest review packages for a language model

**Contest Review** covers finished VPs and official AtCoder contests. Add Markdown notes to a VP, attach local code to individual problems, and export a ZIP with contest data, available submissions, and code. For an official AtCoder contest, choose an account and contest ID to collect problems, the submission timeline, verdicts, and any retrievable code. Both packages include a starting document for a language model.

### Local storage and easy migration

Account settings, sync results, training records, and exports live in the app's data directory. No OJ Insight account is required. Back up or migrate by copying that directory after closing the app.

## What you can see

- **Career and selected-range statistics:** solved problems, accepted submissions, active days, longest and current streaks, and the busiest day.
- **Following notifications:** watch public accounts of teammates or friends and see today's ACs, with one notification per submission.
- **Activity heatmaps:** view a calendar year or the last 365 days using First AC, Unique AC, AC Submissions, or the platform's original Activity count.
- **Difficulty footprint:** retain each judge's own difficulty scale and open the matching problems from a bar or date.
- **Rating overview:** inspect current and peak ratings, recent changes, contest curves, and links to contests where supported.
- **Recent records:** browse recent accepted problems and open their statements.
- **ICPC / CCPC Tracker:** follow contest upsolving and examine problem tiers from public standings.
- **Image export:** export PNG or SVG activity charts by year, metric, and platform.
- **Contest review ZIPs:** package official contests or VPs separately; VP packages may include notes, available submissions, and code you attached.
- **Cross-judge problem sets:** create, sort, import, export, and turn fixed sets into mock contests.
- **Study Assistant and Solve Journal:** time practice, keep notes and mistakes, revisit saved sessions, and export your records.
- **Mock and virtual contests:** independent configurations, pre-contest countdowns, pause controls, Markdown notes, and available AC / WA verdicts.
- **AI training packages:** export a self-contained candidate ZIP and import generated JSON as a set or contest.

## Supported platforms

| Platform | Account ID | Main available data | Notes |
|---|---|---|---|
| Codeforces | Handle | Problem-level ACs, difficulty, Rating | No Cookie required |
| AtCoder | Username | Problem-level ACs, difficulty, Algorithm Rating | Public problem metadata is cached locally |
| Luogu | Username or numeric UID | Submissions or public activity, solved counts, official difficulty | Falls back to aggregates when an endpoint is restricted |
| NowCoder | Numeric user ID | Regular problem ACs, Tracker completion | Tracker data may use an optional Cookie |
| QOJ | Username | Problem-level ACs and ICPC / CCPC upsolving | The full submission list requires a login Cookie |
| LeetCode | Username or `cn:username` | Solved counts and difficulty; international-site contest Rating where available | Some China-site endpoints may use an optional Cookie |

Upstream sites may change their endpoints or limit access. The amount of data available for a platform can therefore change over time. OJ Insight shows the current limitations on its source-status and statistics pages.

## Quick start

1. Download the installer for your operating system from [Releases](https://github.com/Whalica/OJ_Insight/releases/latest).
2. Open **Settings**, enter the judge accounts you want to sync, and save them.
3. Open **Data Sources** and run a full sync for new accounts.
4. Use the incremental or full sync controls to update your records later.

At startup, the app shows cached data first, then syncs configured platforms and checks for updates in the background. Both startup behaviors can be disabled in Settings.

### Generate a contest review package

1. Save an AtCoder account in Settings.
2. Open **Contest Review → Official Contest Review**, select the account, and enter a contest ID or full URL.
3. Check the contest, choose whether to include post-contest practice, and generate the ZIP.

The package always includes `00-START-HERE.md`, `01-CONTEST.md`, `02-PROBLEMS.md`, and `03-SUBMISSIONS.md`. It does not include Cookies, Sessions, local usernames, or local paths. If code or a statement cannot be retrieved, the package names the missing item.

### Create a training contest

1. In **Problem Sets**, paste problem URLs and optionally add roles and notes. Platform and problem ID can be identified offline; title and tags can be entered or fetched on request. Set and contest descriptions support Markdown and LaTeX.
2. Convert a set into an independent contest in **Mock Contests**, or create one directly.
3. In the virtual contest area, save an optional pre-contest countdown. Select **Start VP** to begin it. Pause the clock or record your ideas during the session.
4. For Personalized Training, export a candidate ZIP, upload it to a language model, and import the resulting JSON as a set or contest.
5. Submit on the original judge. Afterwards, attach available local code in **Contest Review** and export a review package.

Personalized candidates exclude locally solved problems and skip interactive, output-only, or explicitly unsuitable daily-practice problems. Manually assembled sets and contests remain under your control. Tags are hidden until AC by default to avoid revealing a problem's intended technique.

The candidate pool estimates a training range per judge from known problem-level difficulty records and rotates among platforms. If a Luogu practice page exposes problem-level completion, sets display it and candidate selection can exclude those problems. That completion list does not add to submission counts or today's progress.

### Which download should I choose?

- **Windows:** the EXE installer with `Windows` in its filename.
- **macOS:** the `universal-MacOS` DMG, supporting Intel and Apple Silicon.
- **Linux:** the AppImage with `Linux` in its filename.

## Accounts and credentials

### QOJ

QOJ requires a login to expose the full submission list:

1. Log in to [QOJ](https://qoj.ac) in your browser.
2. Copy the login Cookie name and value from the browser developer tools. Cookie names may change; do not copy only the value.
3. Enter the full `name=value` pair in OJ Insight's QOJ Cookie field. You can paste multiple pairs separated by semicolons.

```text
name=xxxxxxxx; another_name=yyyyyyyy
```

An expired Cookie is reported as a login problem. A logged-in account with no ACs correctly shows zero. A page-layout or network failure retains the specific error and the previous cache.

### LeetCode China

Prefix China-site usernames with `cn:`:

```text
cn:username
```

If a public endpoint is unavailable, you can enter a Cookie for that site and retry. For the international site, enter the username after `/u/`.

### Credential safety

Cookies and Sessions are login credentials. Do not share a database, full log, or personal-data export containing credentials with someone you do not trust.

The standard personal-information JSON export omits Cookies and Sessions. They are included only when you explicitly choose a full export and confirm the warning. Runtime logs redact the configured Secret.

## How statistics are counted

### Career and selected range

Career uses all locally known history and does not change when you select a year or “To date.” The selected range covers one calendar year or the most recent 365 days through today.

- **Solved:** the sum of distinct problems accepted at least once within each platform; no deduplication across judges.
- **AC Submissions:** the number of accepted submissions available from the source.
- **Active Days:** distinct dates with Activity above zero.
- **Longest Streak:** the longest historical run of active days.
- **Current Streak:** the active-day run ending today.
- **Peak Day:** the day with the highest count under the selected metric.

### Four Activity metrics

- **First AC:** count one on the first date a problem was accepted in the account's history.
- **Unique AC:** count a problem at most once per day, even if accepted repeatedly.
- **AC Submissions:** count every accepted submission.
- **Platform Activity:** use the judge's published daily activity, chiefly for sources that expose only calendar aggregates.

Records with exact timestamps are assigned dates in the selected time zone. When an upstream source provides only `YYYY-MM-DD`, the source date is retained; no submission time is invented.

### Difficulty

Difficulty is ordered, so the app shows histograms while preserving each platform's scale:

- Codeforces: official Rating bands;
- AtCoder: AtCoder Difficulty;
- Luogu: official difficulty levels;
- LeetCode: Easy, Medium, and Hard;
- NowCoder and QOJ: shown only with a reliable difficulty source.

The overview switches between platforms instead of converting their scales into a fictitious common score. Recognized problems without reliable difficulty are explicitly marked **Unrated**.

## Sync and data management

- **Incremental sync:** continue from the last successful point and deduplicate new records; intended for daily use.
- **Full sync:** fetch all data currently available from a platform and replace its cache; useful after missing data or an upgrade.
- **Clear one platform:** remove that judge's synced data while keeping its account settings.
- **Clear all:** remove synced data for all six judges while keeping account settings.

A full sync runs platform by platform. One failure does not stop the others or delete that platform's last successful data. Deleting or renaming an account clears only that account's local records. Re-add a deleted account and sync it again to restore its data.

## Data location and backup

| System | Default location |
|---|---|
| Windows | The directory containing `OJ Insight.exe` |
| macOS | `~/Library/Application Support/com.ojinsight.app/` |
| Linux | Usually `~/.local/share/com.ojinsight.app/` |

The actual path is shown on the app's **About** page. A typical directory looks like:

```text
OJ Insight/
├─ data/oj-insight.sqlite3
├─ exports/
├─ logs/oj-insight.log
└─ webview/
```

Exit the app before copying the whole directory for migration. On Windows, use a location that ordinary users can write to rather than `Program Files`.

Settings can create a database ZIP snapshot while the app is running, after a SQLite integrity check. It includes accounts, sync data, problem sets, contests, and training records. **The ZIP contains account credentials.** It does not include interface preferences, exported files, or WebView data. To restore, exit the app, move the original `data/` folder to a safe place, and extract `data/oj-insight.sqlite3` from the ZIP into the app directory. For a complete migration, exit the app and copy the whole directory.

## Updates and troubleshooting

The app can check for and install signed updates, or you can download a release manually from [Releases](https://github.com/Whalica/OJ_Insight/releases/latest).

For sync problems, check the error on **Data Sources** and `logs/oj-insight.log`. You may attach relevant, verified-redacted log lines to an Issue, but do not upload Cookies, Sessions, or a complete database.

For Gatekeeper prompts with community macOS builds or startup issues under Linux Wayland / niri, see the [build and troubleshooting guide (Chinese)](docs/BUILDING.md).

## Development and contributions

OJ Insight uses Tauri 2, React, TypeScript, Rust, and SQLite. Source development requires Node.js 22+, pnpm 11+, and stable Rust.

```bash
pnpm install --frozen-lockfile
pnpm check
pnpm build
cargo test --locked --manifest-path src-tauri/Cargo.toml
```

See [architecture notes (Chinese)](docs/ARCHITECTURE.md) for module responsibilities and dependency direction. The [build guide (Chinese)](docs/BUILDING.md) covers system packages, three-platform builds, releases, and regression checks. Please [open an Issue](https://github.com/Whalica/OJ_Insight/issues) for upstream-source changes, incorrect statistics, or usability suggestions.
