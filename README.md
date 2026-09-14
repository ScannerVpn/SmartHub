<div align="center">

# SmartHub

**Your second OS for repetitive work.**
سیستم‌عامل دوم شما برای کارهای تکراری 🚀

A lightweight productivity hub behind one global hotkey (`Alt + Space`):
Smart Clipboard · Text Time Machine · Workspace Modes · AI File Declutterer

`Windows 10/11` · `Tauri 2` · `Svelte 5` · `Rust` · `100% local-first`

</div>

---

## معرفی (فارسی)

SmartHub یک هاب بهره‌وری خیلی سبک است که با یک کلید میانبر جهانی (Alt+Space) روی دسکتاپ شما ظاهر می‌شود:

- **Smart Clipboard (رایگان):** ۲۰ آیتم آخر کلیپ‌بورد + تشخیص خودکار لینک/کد/رنگ، تبدیل لینک به عنوان صفحه، جستجو و پین. همه محتوا رمزنگاری‌شده (AES-256-GCM) و فقط روی دستگاه شما.
- **Text Time Machine (رایگان):** نجات متن‌های تایپ‌شده‌ای که به هر دلیلی از دست رفتند — فقط ۱ ساعت آخر، رمزنگاری‌شده، بدون خواندن فیلدهای رمز عبور.
- **Workspace Modes (پرو):** پروفایل‌های نامحدود Focus / Streaming / Coding — باز و بستن برنامه‌ها، والپیپر، صدا، و «مزاحم نشوید» با یک کلیک.
- **AI File Declutterer (پرو):** پاک‌سازی هوشمند Downloads و Desktop — تشخیص فایل تکراری (BLAKE3)، فایل‌های کهنه، اینستالرها و آرشیوهای بزرگ. کاملاً آفلاین؛ مدل LLM فقط در صورت درخواست شما دانلود می‌شود.

**قیمت:** Free برای همیشه + Pro لایسنس مادام‌العمر ۱۹ دلاری (یا ۴.۹۹/ماه).

## Features

| Module | Tier | What it does |
|---|---|---|
| Smart Clipboard | Free | Last 20 clipboard items, encrypted at rest; auto link→title resolution, code/hex-color detection, search & pin |
| Text Time Machine | Free | Snapshots typed text (last 1 hour, encrypted, password fields excluded) — recover with one click |
| Workspace Modes | Pro ($19) | Unlimited system-state profiles: apps, wallpaper, volume, Do Not Disturb; OBS/Discord creator presets included |
| AI File Declutterer | Pro ($19) | Offline folder scan: duplicates (BLAKE3), stale files, installers, large archives; optional GGUF model (Phi-3/Llama-3.2 Q4) downloaded on first use |

**Privacy, verbatim:** no telemetry, no analytics, no crash reporters. The only network calls are (1) license validation once a day and (2) fetching page titles for clipboard links — both visible in the source. Sensitive data is AES-256-GCM encrypted with a key that lives in Windows Credential Manager.

## Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Shell | **Tauri 2** | ~3 MB binary base, WebView2 on Windows, safe IPC |
| UI | **Svelte 5 + TypeScript + TailwindCSS** | Runes-based state, tiny bundle, glassmorphism design system |
| Core | **Rust + Tokio** | all heavy work (watchers, hashing, HTTP) off the UI thread |
| Storage | **SQLite (rusqlite, bundled)** | zero-dependency local DB, WAL mode |
| Crypto | **AES-256-GCM + `keyring`** | data key vaulted in the OS credential store |
| System | **`windows-rs`**, `arboard`, `tauri-plugin-global-shortcut` | UI Automation text capture, clipboard, `Alt+Space` |
| Licensing | **Cloudflare Workers + D1 + KV** → Lemon Squeezy | free-tier-friendly, Merchant-of-Record tax handling |

## Project layout

```
├─ src/                      # Svelte 5 UI (command palette + panels)
│  └─ lib/
│     ├─ components/         # CommandPalette, ClipboardPanel, RecoveryPanel,
│     │                      #   ModesPanel, AICleaner, LicensePanel, SettingsPanel
│     ├─ stores/             # app navigation, license state, toasts (runes)
│     ├─ api/                # typed commands + a full browser mock for dev
│     └─ utils/
├─ src-tauri/
│  ├─ src/
│  │  ├─ core/               # clipboard.rs text_saver.rs hotkey.rs workspace.rs
│  │  │                      #   ai_engine.rs db.rs crypto.rs
│  │  ├─ licensing/          # activation.rs validation.rs offline_grace.rs
│  │  └─ lib.rs              # AppState, command registration, tray, hotkey
│  ├─ tauri.conf.json        # borderless transparent always-on-top window, CSP
│  └─ capabilities/          # least-privilege permissions
└─ server/workers/           # Cloudflare Workers licensing API (activate/validate/webhook) + D1 schema
```

## Getting started

### 1. Frontend only (instant preview — no Rust needed)

```bash
npm install
npm run dev
```

The app detects it is running in a plain browser and swaps in a **rich mock backend**, so every panel works end-to-end. Great for UI iteration and for showing the product before the native side is installed.

### 2. Full desktop app (Windows)

Prereqs: [Rust](https://rustup.rs), [Node 20+](https://nodejs.org), and the [Tauri Windows prerequisites](https://v2.tauri.app/start/prerequisites/) (WebView2 + VS Build Tools).

```bash
npm install
npm run tauri dev      # dev build with hot reload
npm run tauri build    # release: NSIS + MSI installers (~10 MB)
```

On first run the app:
1. creates `%APPDATA%/app.smarthub.desktop/smarthub.sqlite`,
2. generates a 256-bit data key into Windows Credential Manager,
3. seeds the three built-in Workspace profiles,
4. registers `Alt+Space`, then sits in the tray.

### 3. Licensing backend (for going to production)

```bash
cd server/workers
npm install
npx wrangler d1 create smarthub-licenses        # paste id into wrangler.toml
npx wrangler kv namespace create VALIDATION_CACHE
npm run db:migrate
npx wrangler secret put WEBHOOK_SECRET          # from your Lemon Squeezy webhook settings
npm run deploy
```

Then point `API_BASE` in `src-tauri/src/licensing/mod.rs` at the worker's URL (and add the same origin to the CSP in `tauri.conf.json` / `index.html`).

**Lemon Squeezy side:** create the store → add product "SmartHub Pro" (lifetime license variant) → Settings → Webhooks → URL `https://<worker>/webhook`, tick `license_key_created`, `order_created`, `order_refunded`. Payouts via Payoneer/Wise.

### 4. Optional on-device AI

The declutterer's heuristics always work offline. The LLM refinement layer is compile-time optional:

```bash
npm run tauri build -- -- --features local-ai   # enables llama.cpp bindings
```

Model download happens on first AI use (≈2.4 GB GGUF Q4, streamed to disk).

## Security notes

- **Encryption at rest:** clipboard items and rescued text are AES-256-GCM blobs; the key never leaves the OS credential vault.
- **Password safety:** UI Automation `IsPassword` fields are never read (Windows).
- **License keys:** the full key is vaulted (Credential Manager); the DB only keeps `****-****-****-XXXX` + the instance id. The server stores only the SHA-256 hash.
- **Offline grace:** after a successful validation, Pro keeps working 7 days without network; afterwards it falls back to Free until revalidation succeeds.
- **CSP** is locked down (`'self'` + the licensing origin), the window is borderless to prevent URL spoofing inside fake chrome.

## Roadmap

- [x] v0.1 — Free tier (Smart Clipboard, Text Time Machine), palette UI, licensing plumbing
- [ ] v0.2 — Workspace Modes polish: action editor UI, app picker
- [ ] v0.3 — AI Declutterer: LLM classification via GGUF, recycle-bin mover
- [ ] v0.4 — Creator Quick-Launch: audio routing via Windows Audio Session API
- [ ] v0.5 — macOS port (AXUIElement text capture stub is already behind cfg)
- [ ] Launch — Product Hunt + r/SideProject + 30s demo video

## CI

`.github/workflows/build.yml` compiles the release on `windows-latest` (frontend check + NSIS/MSI artifacts) on every push to `main` and on tags.

---

⚖️ MIT License. **Made for people who hate doing the same thing twice.**
