# Tusk 🐘

Full-stack Rust web app to recall learnings via randomized flashcards, synced to Google Drive.

## Overview

**Tusk** is a minimalist flashcard web application built entirely in Rust. It helps users reinforce what they learn by presenting cards at random—no complex SRS algorithms, just straightforward, serendipitous recall. Cards and media assets are stored directly in the user's personal Google Drive.

## Key Features

- **Full-Stack Rust Server**: Single cohesive binary powered by Leptos (SSR + hydration) with zero JavaScript backend framework overhead.
- **Randomized Recall**: Pick flashcards at pure random from your collection.
- **Rich Media & Markdown Support**: Cards formatted in Markdown, supporting text, audio, and video media elements.
- **Google Drive Storage**: User-owned data. Card Markdown files and media live in a designated Google Drive folder.
- **Google OAuth 2.0**: Seamless web sign-in and authorization to access the user's Google Drive.

## Architecture & Tech Stack

- **Framework**: [Leptos](https://leptos.dev/) (Full-stack SSR web framework in Rust)
- **Runtime**: `tokio` / `axum` integration
- **Auth & Storage**: Google OAuth 2.0 Web flow + Google Drive v3 REST API
- **Data Format**: Markdown (`.md`) files with frontmatter metadata + media attachments (audio/video)

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/) (latest stable toolchain)
- `cargo-leptos`:
  ```bash
  cargo install cargo-leptos --locked
  ```
- Google Cloud project with OAuth 2.0 credentials (Drive API scope enabled).

### Configuration

Create a `.env` file in the project root:

```env
GOOGLE_CLIENT_ID="your-client-id"
GOOGLE_CLIENT_SECRET="your-client-secret"
GOOGLE_REDIRECT_URI="http://localhost:3000/api/auth/callback"
```

### Development

Run the development server with live reload:

```bash
cargo leptos watch
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

### Production Build

```bash
cargo leptos build --release
```

## License

MIT
