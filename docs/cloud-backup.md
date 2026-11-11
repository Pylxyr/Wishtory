# Cloud backup

## For users

Back up your history to **your own Google Drive**: sign in once in **Settings, then Cloud backup**, then press **Back up now** or let it run once a day. Restore on any PC with the same Google account and passphrase. It is off until you set it up.

- **Encrypted on your PC first.** You choose a passphrase, and Google only ever stores a file it cannot read. **If you forget the passphrase, the backups cannot be opened**, and nobody can recover them.
- **Wishtory sees nothing else in your Drive.** It asks Google for a single permission: access to its own hidden app folder.
- **Your Google password never reaches Wishtory**, and the sign-in and passphrase are kept in Windows Credential Manager or the Linux Secret Service, never in a file.
- **Restoring only adds.** It never removes a pull. **Disconnect** revokes access and can delete your backups from Drive.

Cloud backup needs a build that includes a Google client ID. If your copy has none, Settings says so and nothing is sent. The rest of this page is for developers.

Optional backup of the wish history to the person's own Google Drive. The code is in
`crates/wishtory-core/src/cloud/`; the Settings panel is `app/lib/features/settings/cloud_backup.dart`.
For what users are told, see the "For users" section above.

## How it works

1. **Sign in.** Settings → Cloud backup asks for a passphrase, opens Google's sign-in page in the browser
   (OAuth 2.0 authorization code flow with PKCE and a loopback redirect, RFC 8252) and waits for the redirect
   on a one-shot listener on `127.0.0.1`. The only permission requested is
   `https://www.googleapis.com/auth/drive.appdata`: Drive's hidden app-data folder.
2. **Back up.** The history is exported as UIGF v4 (the same file as Export), encrypted with the passphrase
   and uploaded as `wishtory-backup-<UTC time>.wshb`. The newest 5 are kept.
3. **Restore.** The chosen file is downloaded, decrypted and merged with the existing import code, which skips
   what is already stored. A restore never deletes a pull.
4. **Automatic backup** is opt-in. `Core::cloud_backup_if_due` runs when the app starts and after a refresh,
   and backs up if the last backup is over 24 hours old. Nothing runs in the background while the app is closed.

## Security design

| Concern | What is done |
| :--- | :--- |
| Google can read the data | It cannot. Backups are encrypted on the PC (`cloud/vault.rs`): Argon2id (64 MiB, 3 passes, 1 lane, fresh 16-byte salt per backup) derives a 256-bit key, ChaCha20-Poly1305 (via `ring`) encrypts, and the whole header is authenticated as associated data. |
| Weak passphrase | At least 10 characters is enforced. A copied file can be attacked offline, so the passphrase is the only protection, and the "For users" section above says so. |
| Malicious backup file | The Argon2 cost in a file is read before hashing, so it is capped (256 MiB, 10 passes, 4 lanes); downloads are capped at 64 MiB; ids from Drive must match `[A-Za-z0-9_-]{6,128}` before they enter a URL. |
| Token theft from disk | Nothing secret is written to a file. The refresh token and the passphrase live in the OS credential store (`keyring`: Windows Credential Manager, or the Secret Service on Linux). If there is none, cloud backup refuses to run instead of falling back to a file. The short-lived access token is kept in memory only. |
| Access beyond Wishtory's files | The `drive.appdata` scope cannot see any other Drive file. |
| Forged sign-in callback | The listener checks the `Host` header, the path and a random `state` (single use, 256 bits), ignores anything else, and gives up after 5 minutes. PKCE protects the code exchange. |
| Open redirect / phishing link | The sign-in URL is built in Rust and must be `https://accounts.google.com/...` before it is opened. It does not go through the Dart link allow-list. |
| Leaking secrets in logs and errors | Nothing secret is logged. Google's error bodies are not shown; HTTP errors become fixed, friendly messages. |
| Leaving a sign-in behind | Disconnect revokes the refresh token with Google and removes both secrets. If Google reports the token as revoked or expired, the local state is cleared too, so the page offers to sign in again. |
| Unwanted uploads | Cloud backup does nothing until the person signs in, and the daily backup is a separate switch. A build without a client ID has the feature disabled. |

Not covered: someone with the passphrase and access to the Google account (they can read the backups, which is the
point), and malware running as the same user (it can read what the user can).

## Setting it up for a build

A desktop app cannot keep a Google client secret confidential, and
[Google's documentation](https://developers.google.com/identity/protocols/oauth2/native-app) says that is expected for installed
apps: security comes from PKCE and the loopback redirect, not from the secret. The ID and secret are still kept out
of the repository, and are compiled in at build time.

1. In the [Google Cloud console](https://console.cloud.google.com/), create a project (any name).
2. **APIs & Services → Library**: enable **Google Drive API**.
3. **OAuth consent screen** (Google Auth platform → Branding / Audience): user type **External**, fill in the app
   name, support email and developer email. Under **Data access** add only the scope
   `.../auth/drive.appdata`. It is a non-sensitive scope, so no Google verification review is needed.
4. **Publish the app** (Audience → *In production*). While it is in *Testing*, Google expires refresh tokens after
   7 days, and backups would silently stop. With only a non-sensitive scope, publishing does not require verification.
5. **Credentials → Create credentials → OAuth client ID → Desktop app.** Copy the client ID and secret.
6. Give them to the build:
   - **GitHub releases:** add repository secrets `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`
     (Settings → Secrets and variables → Actions). `release.yml` passes them to the build as
     `WISHTORY_GOOGLE_CLIENT_ID` and `WISHTORY_GOOGLE_CLIENT_SECRET`. Without them the release is built without cloud
     backup, and the Settings page says so.
   - **Linux, from source:**
     `WISHTORY_GOOGLE_CLIENT_ID=... WISHTORY_GOOGLE_CLIENT_SECRET=... bash packaging/linux/install.sh`
   - **Development:** export the same two variables before `cargo build` / `flutter run`. They are read with
     `option_env!`, so changing them rebuilds the core.

On Linux the credential store must provide the Secret Service API (KDE Wallet and GNOME Keyring do). A minimal
window manager with neither will see "your system's password storage is not available" and cloud backup stays off.

## Removing access

The person can remove Wishtory under Google Account → Security → Third-party access
([myaccount.google.com/permissions](https://myaccount.google.com/permissions)), or press **Disconnect** in the app
(optionally deleting the backups from Drive too).

## Backup file format

```text
"WSHB" | version u8 (1) | kdf u8 (1 = Argon2id) | m_cost_kib u32 | t_cost u32 | p_cost u32 | salt[16] | nonce[12] | ciphertext + 16-byte tag
```

Integers are little-endian. The plaintext is a UIGF v4 JSON file. Changing the version or the format later means a
new `version` byte; older files stay readable.

## Tests

`cargo test -p wishtory-core cloud` covers the container (round trip, wrong passphrase, tampering, hostile cost
parameters), the PKCE example from RFC 7636, the loopback listener (strays, forged state, wrong host, timeout), the
token exchange, and the whole backup / restore / prune / sign-out flow against a fake Google server that runs in the
test. Nothing in the tests talks to Google or to a real credential store.
