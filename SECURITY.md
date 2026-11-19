# Security

## Reporting a problem

Please report a security problem **privately**: use **Security → Report a vulnerability** on this
repository's GitHub page. Do not open a public issue for it. You will get an answer as soon as the
maintainer can read it; a fix is released as a normal update.

## What matters most here

- **Your history link (authkey).** It is read from the game's own cache, sent only to the official
  HoYoverse history service, and never logged, saved or shown. A way to make Wishtory send it
  anywhere else, or write it to a file, is a vulnerability.
- **The signed game catalog.** Catalog, banner and rules updates are accepted only when they carry a
  valid signature from the key built into the app (`catalog/catalog-public-key.txt`). A way to get an
  unsigned or tampered update accepted is a vulnerability.
- **Cloud backup.** Backups are encrypted on your PC before they leave it. A way to read one without
  the passphrase, or to reach anything in your Google Drive other than the app's hidden folder, is a
  vulnerability.

## Supported versions

Only the latest release receives fixes.
