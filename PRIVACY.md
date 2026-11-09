# Wishtory Privacy Policy

_Last updated: 9 November 2026_

Wishtory is a free, open-source desktop app (Windows and Linux) that shows your wish history, pity and 50/50s for Genshin Impact, Honkai: Star Rail and Zenless Zone Zero. This policy explains what happens to your data. The short version: **Wishtory has no servers and no accounts. The developer does not collect, receive or sell any of your data.**

## What stays on your PC

Wishtory reads the history link the game already saved on your computer and stores your wish history, account nicknames and settings in a local database on your PC:

- Windows: `%APPDATA%\app.wishtory.desktop`
- Linux: `~/.local/share/app.wishtory.desktop`

Downloaded item pictures are cached in your system's cache folder. Wishtory only reads game files; it never changes them. Deleting these folders removes everything Wishtory stored. Uninstalling the app does not delete your history automatically.

## Where Wishtory connects to the internet

| Service | Why | What is sent |
| :--- | :--- | :--- |
| HoYoverse history service (official game servers) | To fetch your wish records when you press Refresh | Your game history link, to the official service for that game only |
| GitHub (this repository's releases and files) | Daily check for new app versions, and for banner and item catalog updates | Normal web request data (such as your IP address); no wish or account data |
| enka.network | To download item pictures once | Normal web request data; no wish or account data |

These third parties receive standard request data like your IP address, under their own privacy policies. The developer does not receive any of it.

## Optional Google Drive backup

Cloud backup is **off until you turn it on** in Settings. If you do:

- You sign in on Google's own page in your browser. Your Google password never reaches Wishtory.
- Wishtory requests one permission, `https://www.googleapis.com/auth/drive.appdata`, which gives access only to a hidden app-data folder created for Wishtory. It cannot see, read or change any other file in your Drive.
- Your wish history is **encrypted on your PC** with a passphrase you choose, then uploaded to that folder. Google and anyone with access to your Google account see only an unreadable file. If you forget the passphrase, nobody, including the developer, can recover the backups.
- Wishtory reads your Google account email address only to show which account is connected.
- Your Google sign-in token and passphrase are kept in your system's password storage (Windows Credential Manager or the Linux Secret Service), never in a plain file.
- Backups go straight between your PC and your own Google Drive. They never pass through any server run by the developer.

You can stop at any time: press **Disconnect** in Settings (this revokes access with Google and can also delete your backups from Drive), or remove Wishtory at [myaccount.google.com/permissions](https://myaccount.google.com/permissions).

**Google API Services User Data Policy.** Wishtory's use and transfer of information received from Google APIs to any other app adheres to the [Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy), including the Limited Use requirements. Data from Google is used only to provide the backup and restore feature you asked for. It is not used for advertising, not sold, not shared with anyone, and not read by humans.

## What Wishtory does not do

- No analytics, tracking, advertising or crash reporting.
- No accounts and no data sent to the developer.
- No selling or sharing of personal data.

## Children

Wishtory is not directed at children under 13 and does not knowingly collect information from anyone.

## Changes

If this policy changes, the new version will be published in this repository with an updated date.

## Contact

Questions or concerns: open an issue at [github.com/Pylxyr/Wishtory/issues](https://github.com/Pylxyr/Wishtory/issues).

Wishtory is an unofficial fan project, not affiliated with or endorsed by HoYoverse / COGNOSPHERE or Google.
