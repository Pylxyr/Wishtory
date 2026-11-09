# Wishtory Privacy Policy

_Last updated: 10 November 2026_

Wishtory is a free, open-source desktop app for Windows and Linux. It shows your wish history, pity and 50/50s for Genshin Impact, Honkai: Star Rail and Zenless Zone Zero.

## In short

- **The developer collects nothing.** Wishtory has no accounts, no servers, no analytics, no advertising and no crash reporting. Nothing you do in the app is sent to the developer.
- **Your wish history stays on your PC.** It leaves your PC only if you choose to export it, or turn on the optional Google Drive backup.
- **Cloud backup is off until you turn it on.** If you do, your history is encrypted on your PC with a passphrase only you know, and stored in a hidden folder of your own Google Drive.
- **You can check all of this.** Wishtory is open source, so anyone can read exactly what it does.

## 1. What Wishtory handles

| Data | Where it comes from | Where it is kept | Does it leave your PC? |
| :--- | :--- | :--- | :--- |
| Wish history (pulls, UIDs, banners, times) | The games' official history service, or a file you import | A local database on your PC | Only if you export it, or use cloud backup (encrypted) |
| Game history link | A cache file the game already saved on your PC, which Wishtory only reads, or a link you paste | Not saved by Wishtory | Sent only to the official history service of that game, to fetch your records |
| Settings and preferences (theme, nicknames, game folders) | You | Your PC | No |
| Item pictures | enka.network | A cache folder on your PC | No (they are downloaded, never uploaded) |
| Diagnostics log (optional) | The app, only if you switch it on | Memory only, never written to disk, cleared when switched off | Only if you copy it yourself |
| Google account email, Google sign-in token, backup passphrase | Google, and you (only if you use cloud backup) | See section 3 | See section 3 |

Wishtory only reads game files. It never changes, locks or writes to them.

Where your data lives:

- Windows: `%APPDATA%\app.wishtory.desktop`
- Linux: `~/.local/share/app.wishtory.desktop`

Uninstalling Wishtory does not delete this folder, so your history survives a reinstall. Delete the folder to remove everything Wishtory stored.

## 2. Where Wishtory connects to the internet

| Service | Why | What is sent |
| :--- | :--- | :--- |
| HoYoverse history service (official game servers) | To fetch your wish records when you press Refresh | Your game history link, to the official service for that game only |
| GitHub (this project's releases and files) | A check for new app versions, and for banner and item catalog updates | Standard web request data, including your IP address and the app name and version |
| enka.network | To download item pictures once | Standard web request data. No wish or account data |
| Google (only if you use cloud backup) | Sign-in, and uploading and downloading your encrypted backups | See section 3 |

These services receive standard request data such as your IP address and are covered by their own privacy policies. The developer receives none of it.

## 3. Optional Google Drive backup

Cloud backup is off until you turn it on in **Settings, then Cloud backup**.

**What Wishtory accesses.** One Google permission only: `https://www.googleapis.com/auth/drive.appdata`. It gives access to a hidden app-data folder that exists only for Wishtory. Wishtory cannot see, list, read or change any other file in your Google Drive, and you will not see the backups in the normal Drive view. Wishtory also reads your Google account email address, only to show you which account is connected.

**What it does with it.**

- Your wish history is **encrypted on your PC** before upload, using a key made from a passphrase you choose (Argon2id and ChaCha20-Poly1305). Google, and anyone with access to your Google account, see only an unreadable file.
- Backups go directly between your PC and your own Google Drive. They never pass through any server run by the developer.
- Restoring downloads a backup, decrypts it on your PC and merges it into your history. It never removes existing pulls.

**What is stored, and where.**

- The backup files are stored in your Google Drive's hidden app-data folder. Wishtory keeps the latest 5 and deletes older ones.
- Your Google sign-in token and your backup passphrase are stored in your system's password storage (Windows Credential Manager, or the Linux Secret Service such as KWallet or GNOME Keyring). They are never written to a plain file. If your system has no password storage, cloud backup stays off.
- Your Google account email address and your backup preferences (automatic backup on or off, time of the last backup) are stored in Wishtory's settings file on your PC.

**Your passphrase.** Wishtory cannot recover it, and neither can the developer. If you forget it, your existing backups cannot be opened.

**Sharing.** Data received from Google is not shared with anyone, sold, used for advertising, used to build profiles, or read by humans.

**Google API Services User Data Policy.** Wishtory's use and transfer of information received from Google APIs to any other app adheres to the [Google API Services User Data Policy](https://developers.google.com/terms/api-services-user-data-policy), including the Limited Use requirements. Data from Google is used only to provide the backup and restore feature you asked for.

## 4. Security

- Backups are encrypted before they leave your PC. Each backup uses its own random salt, and any change to a backup file is detected when it is opened.
- All connections use HTTPS.
- Google sign-in happens on Google's own page in your browser, so your Google password never reaches Wishtory.
- The diagnostics log removes history links, tokens and passphrases from anything it records.

No software can promise perfect security. If you find a problem, please report it (see Contact).

## 5. Your choices and how to delete your data

| To | Do this |
| :--- | :--- |
| Delete your history on this PC | Delete the data folder shown in section 1, or use **Clear all saved history** on the Accounts page |
| Stop cloud backup | Press **Disconnect** in Settings, then Cloud backup. This revokes Wishtory's access with Google and removes your sign-in and passphrase from your PC |
| Delete your backups from Google Drive | Choose **Disconnect and delete backups**. Google Drive's settings (Manage apps) can also delete an app's hidden data |
| Remove Wishtory's access from outside the app | Open [myaccount.google.com/permissions](https://myaccount.google.com/permissions) and remove Wishtory |
| Move your history elsewhere | Use **Export**. You get a standard UIGF file |

The developer holds none of your data, so there is nothing to request from the developer. Everything above is under your control.

## 6. What Wishtory does not do

- No analytics, tracking, advertising or crash reporting.
- No accounts, and no data sent to the developer.
- No selling or sharing of personal data.
- No reading, changing or locking of game files, and no interaction with the running game.

## 7. Children

Wishtory is not directed at children under 13 and does not knowingly collect information from anyone.

## 8. Changes

If this policy changes, the new version is published in this repository with an updated date, and you can see every change in the repository history.

## 9. Contact

Questions, concerns or security reports: open an issue at [github.com/Pylxyr/Wishtory/issues](https://github.com/Pylxyr/Wishtory/issues).

Wishtory is an unofficial fan project, not affiliated with or endorsed by HoYoverse / COGNOSPHERE or Google.
