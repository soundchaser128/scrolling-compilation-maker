# Scrolling Compilation Maker

Make a vertical scrolling compilation from an Alexandria media pool. Select clips, choose a time range in each, and render a video to download. Each selected segment loops while the viewport scrolls across the strip. Video trims use stream copy, so the start and end can land on nearby keyframes.

## Requirements

Install Rust, Node.js and npm, and ffmpeg (including ffprobe). Install yt-dlp if you want to add a song from a URL. The configured Alexandria server and content storage must be reachable, including from the browser for video previews.

## Development

Build the frontend once before compiling the server, because the server embeds its assets:

```sh
cd web
npm ci
npm run build
cd ..
cargo run --manifest-path server/Cargo.toml
```

For frontend development, run `npm run dev` in `web` in another terminal and open the displayed Vite URL. Vite forwards `/api` requests to `127.0.0.1:3001`. The server listens on that address by default.

## Production

Build the frontend first, then the server:

```sh
npm ci --prefix web
npm run build --prefix web
cargo build --release --manifest-path server/Cargo.toml
./server/target/release/scm-server
```

The binary embeds the built frontend and serves the UI and API from one origin; no frontend files or Node runtime are needed at runtime. Open `http://127.0.0.1:3001`.

## Configuration

| Environment variable | Default | Purpose |
| --- | --- | --- |
| `ALEXANDRIA_API_URL` | `https://alexandria.soundchaser128.com` | Alexandria API base URL |
| `CONTENT_URL` | `https://content.r2.soundchaser128.com` | Media content base URL |
| `BIND_ADDR` | `127.0.0.1:3001` | HTTP listen address |
| `OUTPUT_DIR` | `./output` | Directory for rendered MP4 files |
| `GPU` | `false` | Set to `true` to use GPU encoding by default |

Choose filters and generate a pool, select at least two clips and adjust their ranges, then choose render settings and download the finished file. Jobs and selections are held in memory and are lost when the server or page is restarted.
