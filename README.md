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

The following demonstrates the SMS login process with the help of MessAuto:

https://github.com/LeeeSe/MessAuto/assets/44465325/6e0aca37-377f-463b-b27e-a12ff8c1e70b

🎉🎉🎉 MessAuto now supports the Mail app

🎉🎉🎉 MessAuto now supports DingTalk (via macOS Notification Center)

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
- Listen for DingTalk: When enabled, extracts verification codes from DingTalk banner notifications by reading the macOS Notification Center database (requires Full Disk Access, and DingTalk notifications must be turned on in System Settings -> Notifications). This does not read DingTalk's own encrypted database.
- Hide Icon: Temporarily hides the menu bar icon. The icon will reappear when the app restarts (you can stop it via Activity Monitor).
- Config: Quickly opens the TOML configuration file where you can customize regex patterns and keywords.
- Log: Quickly opens the log file.
- Floating Window: Automatically shows a popup near the cursor when a verification code is detected. This forces the "Don't Occupy Clipboard" mode.

> Keywords: Also called trigger words. The program will only perform subsequent operations when a message contains keywords such as "verification code". Otherwise, the message will be ignored.

## FAQ
### Cannot Open Because the Developer Cannot Be Verified
Two solutions (the second is recommended):
- Open Finder, locate MessAuto.app, and right-click to open it.
- Run `sudo spctl --master-disable` in Terminal; then go to System Settings -> Privacy & Security -> Allow applications from -> Anywhere.

## TODO

- [x] Add in-app updates
- [x] Optimize verification code extraction logic
- [ ] Publish to Homebrew
- [ ] Support third-party email clients
- [ ] Support Android and Windows

## Development

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
