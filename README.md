<p align="center">
  <img src="resources/icon_256.png" height="256">
  <h1 align="center">MessAuto</h1>
  <h4 align="center"> Automatic extraction of SMS and email verification codes on Mac</h4>
<p align="center">
<a href="https://github.com/LeeeSe/MessAuto/blob/master/LICENSE.txt">
<img src="https://img.shields.io/github/license/LeeeSe/messauto"
            alt="License"></a>
<a href="https://github.com/LeeeSe/MessAuto/releases">
<img src="https://img.shields.io/github/downloads/LeeeSe/messauto/total.svg"
            alt="Downloads"></a>
<a href="https://img.shields.io/badge/-macOS-black?&logo=apple&logoColor=white">
<img src="https://img.shields.io/badge/-macOS-black?&logo=apple&logoColor=white"
            alt="macOS"></a>
</p>

<p align="center">
  [<a href="./README.md">English</a>] [<a href="docs/README-CN.md">中文</a>]<br>
</p>

# MessAuto

MessAuto is a macOS application that automatically extracts SMS and email verification codes. Built with Rust, it works with any app.

Authenticated HTTP Webhooks and an embedded ngrok tunnel allow Android SMS forwarding apps to send verification messages directly to your Mac.

The following demonstrates the SMS login process with the help of MessAuto:

https://github.com/LeeeSe/MessAuto/assets/44465325/6e0aca37-377f-463b-b27e-a12ff8c1e70b

🎉🎉🎉 MessAuto now supports the Mail app

https://github.com/LeeeSe/MessAuto/assets/44465325/33dcec87-61c4-4510-a87c-ef43e69c4e9d

## Usage
### Download and Install
Download from [Releases](https://github.com/LeeeSe/MessAuto/releases/latest)
### Permissions
- Full Disk Access: System Settings -> Privacy & Security -> Full Disk Access
- Accessibility: System Settings -> Privacy & Security -> Accessibility
### Keep Messages and Mail Apps Running in the Background
Keeping these apps running in the background shortens the time it takes to sync messages from iPhone to Mac.

## Configuration
- Auto Paste: Simulates keyboard input to automatically paste the verification code from the clipboard into the input field.
- Auto Return: Automatically presses the Return key after pasting the verification code.
- Don't Occupy Clipboard: MessAuto simulates keyboard input to directly type the verification code without occupying the clipboard.
- Listen for SMS: When enabled, monitors the built-in Messages app on Mac (works best when the app is running in the background; otherwise there may be a delayed response — this is not a MessAuto issue).
- Listen for Email: When enabled, monitors the built-in Mail app on Mac (same as above).
- Hide Icon: Temporarily hides the menu bar icon. The icon will reappear when the app restarts (you can stop it via Activity Monitor).
- Config: Quickly opens the TOML configuration file where you can customize regex patterns and keywords.
- Log: Quickly opens the log file.
- Floating Window: Automatically shows a popup near the cursor when a verification code is detected. This forces the "Don't Occupy Clipboard" mode.

> Keywords: Also called trigger words. The program will only perform subsequent operations when a message contains keywords such as "verification code". Otherwise, the message will be ignored.

## Webhook and embedded ngrok

Webhook is enabled by default at `0.0.0.0:8978`. POST to `/webhook` (or `/`) with the full message in `value`. It uses the same parser, floating window, direct input, clipboard, paste and Return settings as Messages and Mail.

On first launch, MessAuto generates a random Webhook token. Copy it from the **Webhook** tray submenu:

```bash
TOKEN='<your Webhook token>'
curl -X POST 'http://127.0.0.1:8978/webhook' \
  -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"value":"Your verification code is 131964"}'

# Alternatively, pass the token in the URL:
curl -X POST "http://127.0.0.1:8978/webhook?token=$TOKEN" \
  -H 'Content-Type: application/json' \
  -d '{"value":"Your verification code is 131964"}'
```

A match returns `{"ok":true,"code":"131964"}` and queues the configured action. A message without a code returns HTTP 200 with `ok:false`; invalid JSON or a missing `value` returns 400; missing/incorrect credentials return 401. The Authorization header takes precedence over the query token.

The tray submenu lets you enable Webhook/ngrok, view their status, copy public or LAN URLs (including the token), copy/regenerate the Webhook token, and set the ngrok authtoken in a hidden input dialog. Regenerating the token invalidates old forwarding URLs. LAN address discovery tries `en0`, then `en1`.

ngrok runs inside MessAuto via the Rust SDK; no CLI installation is required. It is enabled by default when Webhook is running. If no authtoken is configured, MessAuto tries to import it from `~/Library/Application Support/ngrok/ngrok.yml` or `~/.config/ngrok/ngrok.yml`; otherwise choose **Set ngrok Token…**. Set `ngrok_domain` in `config.toml` to the dev domain assigned to your ngrok account before connecting. If it is empty, the tray and a notification explain how to configure it; the local Webhook keeps running. SDK-generated random URLs cannot be fixed and reused on the free plan.

**Stop any external ngrok CLI using the same domain before connecting.** MessAuto will notify you of conflicts; it does not kill other processes. Disabling ngrok keeps the local Webhook running.

Configuration is stored in `~/Library/Application Support/messauto/config.toml`:

```toml
listen_webhook = true
webhook_host = "0.0.0.0"
webhook_port = 8978
webhook_token = "<generated token>"
ngrok_enabled = true
ngrok_authtoken = "<ngrok account authtoken>"
ngrok_domain = "<your-assigned-domain>.ngrok-free.dev"
version = 2
```

Existing settings are preserved when upgrading to configuration version 2. After editing the domain, toggle ngrok off/on; restart MessAuto after editing the listener address or port.

Treat both tokens and copied URLs as secrets. The ngrok authtoken is stored as plain text in the configuration file. Public ngrok URLs use HTTPS; local HTTP traffic is unencrypted, so use a trusted LAN. Prefer the Bearer header where possible to avoid putting the Webhook token in URL history/logs. Authorized requests can trigger keyboard input according to your current settings.

## FAQ
### Cannot Open Because the Developer Cannot Be Verified
Two solutions (the second is recommended):
- Open Finder, locate MessAuto.app, and right-click to open it.
- Run `sudo spctl --master-disable` in Terminal; then go to System Settings -> Privacy & Security -> Allow applications from -> Anywhere.

### Accessibility Is Enabled, but Input Still Fails
An old permission entry can refer to a previous build's code signature. This was observed during local verification: the macOS TCC log reported a code requirement mismatch even though the switch was on.

Quit MessAuto, remove its stale Accessibility entry (or run the app-specific command below), add `/Applications/MessAuto.app` again in **System Settings → Privacy & Security → Accessibility**, enable it, and reopen the app:

```bash
tccutil reset Accessibility com.leeese.messauto
```

Full Disk Access is a separate permission used to read Messages/Mail; Accessibility permits keyboard input. When macOS asks to quit and reopen after granting access, make sure it reopens the current app in `/Applications`.

### Webhook Cannot Listen on Port 8978
Quit another app using the same port (such as AutoCode), or change `webhook_port` in `config.toml` and restart MessAuto.

## TODO

- [x] Add in-app updates
- [x] Optimize verification code extraction logic
- [ ] Publish to Homebrew
- [ ] Support third-party email clients
- [ ] Support Android and Windows

## Development

Update checks use the official `cargo-packager-updater` 0.2.3. To use a proxy, set `HTTP_PROXY`, `HTTPS_PROXY` or `ALL_PROXY` in the environment before launching MessAuto; macOS System Settings proxy detection from the former custom fork is no longer used.

```bash
# Clone the source code
git clone https://github.com/LeeeSe/MessAuto.git
cd MessAuto

# Build and run (optional, for development and testing only)
cargo run

# Install cargo-packager
cargo install cargo-packager --locked
# Build
cargo build --release
# Package the application
cargo packager --release
```

The generated MessAuto application is located at `target/release/MessAuto.app`.

## Acknowledgments

- Thanks to [@尚善若拙](https://sspai.com/post/73072) for providing the idea of extracting SMS verification codes.
