---
title: The Windows taskbar turns black when PhotoSoft is maximized
description: On Windows 11, PhotoSoft 0.7.0 can turn the taskbar black or hide it when maximized. PhotoSoft 0.8.2 fixes this; update.
order: 10
verified: 0.8.2
---

Update PhotoSoft. Version 0.7.0 on Windows 11 could turn the taskbar black or hide it while the window was maximized. PhotoSoft 0.8.2 includes the fix.

## Symptoms

- You maximize the PhotoSoft window, or switch it to full screen, and the Windows taskbar turns completely black or is hidden.
- After you resize the window and maximize it again, a black bar the height of the taskbar stays at the bottom of the screen.

## Fix

1. Check your version with **Help › System Info…**, which starts with "PhotoSoft" and the version.
2. If it is older than 0.8.2, download the [latest release](https://github.com/storytold/photocraft/releases/latest) or update with [ArtCraft Launcher](https://getartcraft.com/docs/craft-launcher). See [Install PhotoSoft](../getting-started/install.md).
3. Start PhotoSoft again and maximize the window.

A user who saw the black taskbar on 0.7.0 confirmed that it is gone in 0.8.2 ([issue 3864](https://github.com/storytold/photocraft/issues/3864)).

## Why this happens

PhotoSoft draws its own title bar. When such a window was maximized, Windows sized it to cover the whole screen, taskbar included, took it for a full-screen app and dropped the taskbar's background. The fix keeps a maximized PhotoSoft window clear of the taskbar ([issue 3144](https://github.com/storytold/photocraft/issues/3144), [pull request 3308](https://github.com/storytold/photocraft/pull/3308)).

## Still stuck?

If you still see it on 0.8.2, add your Windows version and PhotoSoft version to [issue 3864](https://github.com/storytold/photocraft/issues/3864), or ask in [Discord](https://discord.gg/artcraft). As a test, you can switch on **System title bar** under **Edit › Preferences › Interface…**, click **Apply** and restart PhotoSoft; Windows then draws the title bar. See [Get help with a problem](get-help.md) for what to include.
