# Tusk

Full-stack Rust web app for single-sided flashcard recall (text notes, audio, video, images), stored directly in Google Drive.

## Overview

**Tusk** is a minimalist flashcard app built entirely in Rust. Flashcards are single-sided—either a text note or a single media file (audio, video, image)—drawn purely at random for serendipitous recall. All data is saved inside the user's Google Drive.

## Flashcard Folder Structure

Every flashcard is its own subfolder inside the selected Google Drive deck folder, named after the card title, containing an `index.<ext>` file:

```text
Google Drive Deck/
├── guitar-scale/
│   └── index.mp3       # Audio card
├── cell-mitochondria/
│   └── index.md        # Text note card
├── workout-form/
│   └── index.mp4       # Video card
└── architectural-sketch/
    └── index.png       # Image card
```

- **Card Title**: Derived from the folder name.
- **Card Content**: Exactly 1 side: 1 text note (`index.md`) or 1 media file (`index.<ext>`).
- **Supported Formats**:
  - Text: `.md`
  - Audio: `.mp3`, `.wav`, `.ogg`, `.m4a`, `.aac`
  - Video: `.mp4`, `.webm`, `.mov`, `.mkv`
  - Images: `.png`, `.jpg`, `.jpeg`, `.webp`, `.gif`, `.svg`

## Key Features

- **Single-Sided Recall**: No flip mechanics—direct review of note or media player + Next Card draw.
- **Instant Local Preview**: Local blob URL preview for selected audio/video/image before submitting.
- **Upload on Submit**: Media and card folder created atomically in Drive on save without file size limits.
- **Secure Streaming Proxy**: Authenticated backend proxy (`/api/media/file/:file_id`) with `Range` header support for smooth audio/video scrubbing without public Drive permissions.
- **Full-Stack Rust**: Leptos SSR + hydration with Axum backend.

## Tech Stack

- **Framework**: [Leptos](https://leptos.dev/) (0.6, SSR + Hydration)
- **Backend / Routing**: Axum 0.7 + Tokio
- **Auth & Storage**: Google OAuth 2.0 + Google Drive v3 API
- **Styling**: Tailwind CSS

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable)
- `cargo-leptos`:
  ```bash
  cargo install cargo-leptos --locked
  ```
- Google Cloud project with OAuth 2.0 credentials and Drive API enabled.

### Configuration

Create a `.env` file in the project root:

```env
GOOGLE_CLIENT_ID="your-client-id"
GOOGLE_CLIENT_SECRET="your-client-secret"
GOOGLE_REDIRECT_URI="http://localhost:3000/api/auth/callback"
SESSION_SECRET="your-32-byte-secret-key-here"
```

### Development

```bash
cargo leptos watch
```

Open [http://localhost:3000](http://localhost:3000).

### Production Build

```bash
cargo leptos build --release
```

## License

MIT
