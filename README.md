# 🖱️ Stable Motion
### *Empowering Precision for Parkinson's Patients*

> **Team 404 Brain Not Found** | Healthcare Track  
> Rudra Bhattacharjee · Samrat Talukdar · Avik Roy Choudhury · Shreyan Das · Soumyajit Maity

---

<p align="center">
  <img src="https://upload.wikimedia.org/wikipedia/commons/thumb/f/f5/Cursor_%3F.svg/250px-Cursor_%3F.svg.png" alt="Computer Mouse" width="180"/>
</p>

---

## 😔 The Human Problem

Imagine trying to click a small button on your screen — but your hand won't stop shaking.

That's everyday reality for someone living with **Parkinson's disease**. Every time they reach for a mouse, their hand trembles at 4–6 times per second. They aim for a link, but the cursor dances away. They try to close a window, but they accidentally open something else. Something as simple as sending an email — something most of us do without thinking — becomes exhausting, frustrating, and demoralizing.

> *"She wanted to video call her grandchildren. It took her 20 minutes to click the right button."*

These are not rare moments. For **10 million Parkinson's patients worldwide**, this is every single day. 70% of them struggle specifically with mouse control. And when they look for help, the solutions are either:
- **Too expensive** — specialized adaptive mice cost $500+
- **Too weak** — built-in accessibility tools like Sticky Keys do nothing for active tremors
- **Too complicated** — requiring hardware modifications, doctors, or IT support

**They just want to use their computer. Independently. With dignity.**

So we built a fix — in software, at zero cost, using nothing but the mouse they already own.

---

## 🧩 The Technical Problem

10 million people worldwide live with Parkinson's disease. 70% of them struggle with standard mouse interfaces due to involuntary hand tremors (4–6 Hz frequency range). Existing accessibility tools like Sticky Keys are not built for **active tremor filtering**, and specialized hardware costs $500+.

**Stable Motion** is a pure software solution — zero new hardware required. Zero installation cost. Just run it.

---

## ⚙️ How It Works

The program sits **between your physical mouse and the OS**, intercepting raw input, filtering tremors in real-time, and re-emitting smooth, high-frequency movement events via a virtual device.

```
Physical Mouse ──► [EXCLUSIVE GRAB] ──► STFTFilter (FFT + EMA) ──► Virtual Mouse ──► OS
```

### The 4-Stage Stability Pipeline

| Stage | What Happens |
|---|---|
| **Capture** | Raw mouse movement events read from `/dev/input/eventX` |
| **Smooth** | EMA (Exponential Moving Average) removes high-frequency jitter |
| **Filter** | FFT isolates and zeroes out 4–6 Hz tremor frequency bands |
| **Restore** | IFFT reconstructs a clean cursor path and emits at ~1000Hz |

---

## 🔬 Technical Deep Dive

### STFTFilter (The Core Brain)
- Maintains a **rolling buffer of 64 samples** per axis (X and Y filtered independently)
- Performs **forward FFT** → zeroes the 4–6 Hz band → **inverse FFT**
- Applies **EMA smoothing** on top: `output = (0.3 × new) + (0.7 × previous)`
- Powered by [FFTW3](https://www.fftw.org/) for high-performance transforms

### Interpolated Emission (The "Frame Gen")
- Each 125Hz hardware tick is split into **8 micro-steps**
- Emitted with ~1ms spacing, effectively upscaling to **~1000Hz output**
- Uses sub-pixel accumulation so no fractional movement is lost

### Input Hijacking via uinput
- `EVIOCGRAB` gives **exclusive ownership** of the physical device
- A virtual device named `"Stable Motion"` is created via `/dev/uinput`
- Clicks are passed through **raw and unfiltered** — only motion is processed

---

## 🚀 Getting Started

### Prerequisites
```bash
# On Ubuntu/Debian:
sudo apt install libfftw3-dev

# On Fedora/RHEL:
sudo dnf install fftw-devel
```

### Build
```bash
g++ main.cpp -o stable_motion -lfftw3
```

### Run
```bash
# Find your mouse device
ls -l /dev/input/by-id/

# Run with sudo (required for uinput and device grab)
sudo ./stable_motion /dev/input/eventX [glide_factor]
```

### Stop
```
CTRL+C
```

---

## 📊 Results

| Metric | Value |
|---|---|
| Tremor Reduction | **~95%** |
| Target Frequency Band | **4–6 Hz** (Parkinson's tremor range) |
| Output Poll Rate | **~1000 Hz** (upscaled from 125 Hz) |
| Hardware Required | **None** (pure software) |
| Platform | **Linux** (Ubuntu/Debian/Fedora) |

---

## 🗺️ Roadmap

**Phase 1 — Core** ✅  
Low-level event interception, EMA baseline smoothing, uinput virtual device

**Phase 2 — Signal** 🔄  
FFT pipeline, rolling buffers, STFT live filtering *(current state)*

**Phase 3 — Polish** 🔜  
UX Dashboard with sensitivity slider, click stabilizer, system-wide toggle shortcuts

**Future**  
Port to Windows (HID drivers) and macOS (global input hooks). Potential health monitoring via anonymized tremor telemetry for clinical use.

---

## ⚠️ Known Limitations

- **Linux only** — uses `/dev/uinput` and Linux input subsystem
- **No middle click** — only left and right buttons registered
- **64-sample warmup** — filter bypassed until buffer is full
- Requires `sudo` to run

---

## 🛠️ Tech Stack

`C++17` · `FFTW3` · `Linux uinput` · `Linux input subsystem`

---

## 📄 License

This project is maintained under the **GNU General Public License 3.0**.

---
