# OpenFairFlow

**A unified workflow orchestration platform for Privacy-Preserving Deep Learning with built-in Explainable AI (XAI) diagnostics. Bridging the gap between decentralized model training and model interpretability.**

---

## Overview

OpenFairFlow is a fullstack application built with **Dioxus 0.7** (Rust) that provides an end-to-end environment for managing datasets, creating collaborative workspaces, and running federated learning experiments - all from an intuitive web interface. It combines privacy-preserving distributed training (FedAvg) with explainability tooling so practitioners can both train and understand their models.

## Features

### Dataset Management
- **Upload** local files or **download** from any URL / HuggingFace Hub
- Automatic metadata extraction (format, size, samples, classes)
- Tagging and rich descriptions for organization
- Status tracking (ready, uploading, downloading, error)

### Dataset Exploration
- **Image preview thumbnails** auto-generated from image datasets
- **Class distribution** histogram visualization
- **Paginated table viewer** for Parquet data with multi-split support (train / test / validation)
- Auto-detection of column types (Image, Text, Number)

### Federated Learning Workspaces
- Create workspaces linked to any dataset
- **Integrated Python code editor** with syntax highlighting (CodeMirror)
- **4 model presets:** MLP, CNN (LeNet-style), ResNet-18 (Mini), and Custom
- **IID and Non-IID** data distribution strategies
- Configurable training parameters: clients, rounds, learning rate, batch size
- Complete **FedAvg algorithm** implementation with weighted averaging

### Visualization & XAI
- Training loss curves per round
- Global test accuracy trajectory
- Per-client accuracy bar charts
- **Explainable AI (XAI) plots** for model interpretability
- HTML table output for detailed results

### Cross-Platform
- Web, Desktop, and Mobile build targets via Dioxus
- Responsive dark-themed UI with mobile hamburger menu

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend & Fullstack | [Dioxus 0.7](https://dioxuslabs.com) (Rust) |
| Styling | Tailwind CSS (auto-compiled) |
| Database | SQLite via `rusqlite` |
| Async Runtime | Tokio |
| Data Formats | Apache Arrow / Parquet |
| ML Execution | Python subprocess (PyTorch) |
| Dataset Hub | HuggingFace Hub API |
| Code Editor | CodeMirror |

## Prerequisites

- **Rust** (stable toolchain)
- **Python 3.8+** with PyTorch, matplotlib, pandas, Pillow, numpy
- [Dioxus CLI](https://dioxuslabs.com/learn/0.7/getting_started):
  ```bash
  curl -sSL http://dioxus.dev/install.sh | sh
  ```

## Getting Started

### 1. Clone the repository

```bash
git clone https://github.com/lab-secureai/OpenFairFlow
cd OpenFairFlow
```

### 2. Install Python dependencies

```bash
pip install torch torchvision matplotlib pandas pillow numpy scikit-learn shap
```

### 3. Run the application

```bash
dx serve
```

This starts the fullstack app (server + web client). Tailwind CSS is compiled automatically.

To target a specific platform:

```bash
dx serve --platform web
dx serve --platform desktop
```

## Routes

| Path | Page | Description |
|------|------|-------------|
| `/` | Home | Dashboard with stats, recent datasets & workspaces |
| `/datasets` | Datasets | Browse, upload, or import datasets |
| `/datasets/:id` | Dataset Detail | View metadata, preview images, explore data tables |
| `/workspaces` | Workspaces | Create and manage federated learning workspaces |
| `/workspaces/:id` | Workspace Detail | Edit code, configure training, run experiments, view results |