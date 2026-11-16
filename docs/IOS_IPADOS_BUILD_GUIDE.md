# Nova 3D Studio — iOS & iPadOS Build Guide (Without a Mac)

This guide explains how to build and install **Nova 3D Studio** on an iPad or iPhone using **GitHub Actions' free cloud macOS runners**, entirely from your Windows PC.

---

## 1. Project Architecture on iOS & iPadOS

Nova 3D Studio utilizes a shared cross-platform architecture:
* **Core Engine (`rust/nova_core`)**: Pure Rust ECS, 3D mesh geometry, bmesh topology (extrude, bevel, loop cut, subdivide), keyframe animation, and camera logic. Compiles directly for Apple Silicon (`aarch64-apple-ios`).
* **Metal 3D Viewport (`ios/Nova3D/Renderer`)**: High-performance MetalKit viewport supporting **Apple Pencil** pressure, hover, single-finger orbit, 2-finger pan/zoom, and **120Hz ProMotion** on iPad Pro.
* **Desktop-Class iPadOS UI (`ios/Nova3D/UI`)**: Full 3-Zone workstation (left tool rail, center 3D viewport with 3D orientation cube, right accordion properties panel, and bottom interactive timeline scrubber). Supports iPadOS **Stage Manager** and split-view multitasking.

---

## 2. One-Time Setup: Push to GitHub

From your Windows terminal (`d:\My Apps\Nova 3d`):

```bash
# 1. Initialize git (if not already done)
git init
git branch -M main

# 2. Add all files and commit
git add .
git commit -m "feat: Add iOS and iPadOS native support with GitHub Actions IPA pipeline"

# 3. Link to your GitHub repository (replace with your repo URL)
git remote add origin https://github.com/YOUR_USERNAME/Nova-3D.git

# 4. Push to GitHub
git push -u origin main
```

---

## 3. How the Cloud Build Works (100% Free)

The workflow `.github/workflows/build-ios.yml` is configured to build on **genuine Apple Silicon (M2) hardware**:

1. Open your repository on **GitHub.com**.
2. Click on the **Actions** tab at the top.
3. Select **"Build Nova 3D iOS & iPadOS (IPA)"** on the left.
4. Click **"Run workflow"** ➔ **"Run workflow"**.
5. In ~3–4 minutes, the job completes with a green checkmark!
6. Click on the completed run, scroll to the bottom under **Artifacts**, and click **`Nova3D-iPadOS-iOS`** to download your `Nova3D.ipa` file to your Windows PC.

---

## 4. Install the IPA onto your iPad or iPhone from Windows

You can install the `.ipa` onto your device without any Mac using **Sideloadly** (Free Windows app):

1. **Download & Install Sideloadly** on Windows:
   * Official site: [https://sideloadly.io](https://sideloadly.io)
2. **Connect your iPad / iPhone** to your Windows PC with a USB cable (tap "Trust This Computer" on the screen).
3. Open **Sideloadly**:
   * It will automatically detect your connected iPad/iPhone.
   * Drag & drop the downloaded `Nova3D.ipa` file into the large IPA box.
   * Enter your free Apple ID email.
   * Click **Start**.
4. Once Sideloadly shows `Done.`, the **Nova 3D Studio** icon will appear on your iPad home screen!
5. On your iPad, go to **Settings ➔ General ➔ VPN & Device Management** and tap **Trust [Your Apple ID]** once to launch the app.
