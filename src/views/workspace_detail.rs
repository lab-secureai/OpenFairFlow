use crate::Route;
use crate::models::ExecutionResult;
use crate::server::{
    delete_workspace_server, get_last_run_result_server, get_workspace_server, run_python_server,
    save_workspace_code_server,
};
use dioxus::prelude::*;

const WORKSPACES_CSS: Asset = asset!("/assets/styling/workspaces.css");

#[component]
pub fn WorkspaceDetail(id: String) -> Element {
    let ws_id = id.clone();
    let workspace = use_server_future(move || {
        let id = ws_id.clone();
        async move { get_workspace_server(id).await }
    })?;

    rsx! {
        document::Link { rel: "stylesheet", href: WORKSPACES_CSS }

        document::Script { src: "https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.18/codemirror.min.js" }
        document::Link {
            rel: "stylesheet",
            href: "https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.18/codemirror.min.css",
        }
        document::Link {
            rel: "stylesheet",
            href: "https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.18/theme/material-darker.min.css",
        }

        div { class: "max-w-7xl mx-auto px-4 py-8",
            match workspace() {
                Some(Ok(Some(ws))) => rsx! {
                    WorkspaceEditor { workspace: ws, id: id.clone() }
                },
                Some(Ok(None)) => rsx! {
                    div { class: "text-center py-20",
                        p { class: "text-[#888] text-lg", "Workspace not found" }
                        Link {
                            to: Route::Workspaces {},
                            class: "text-white hover:underline mt-4 inline-block",
                            "← Back to Workspaces"
                        }
                    }
                },
                Some(Err(e)) => rsx! {
                    div { class: "text-center py-20",
                        p { class: "text-[#ff3333] text-lg", "Error: {e}" }
                    }
                },
                None => rsx! {
                    div { class: "text-center py-20",
                        div { class: "text-2xl animate-spin mb-2", "⟳" }
                        p { class: "text-[#888]", "Loading workspace..." }
                    }
                },
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Model preset info (used for summary card)
// ---------------------------------------------------------------------------

struct ModelInfo {
    name: &'static str,
    params: &'static str,
    layers: Vec<(&'static str, &'static str)>,
}

fn get_model_info(preset: &str) -> ModelInfo {
    match preset {
        "cnn" => ModelInfo {
            name: "CNN (LeNet-style)",
            params: "~62 K",
            layers: vec![
                ("Conv2d(1→32, 3×3)", "30×30"),
                ("ReLU + MaxPool2d(2)", "15×15"),
                ("Conv2d(32→64, 3×3)", "13×13"),
                ("ReLU + MaxPool2d(2)", "6×6"),
                ("Flatten", "2304"),
                ("Linear(2304→128)", "128"),
                ("ReLU", "128"),
                ("Linear(128→10)", "10"),
            ],
        },
        "resnet" => ModelInfo {
            name: "ResNet-18 (Mini)",
            params: "~270 K",
            layers: vec![
                ("Conv2d(1→16, 3×3)+BN", "32×32"),
                ("ResBlock(16→16) ×2", "32×32"),
                ("ResBlock(16→32) ×2", "16×16"),
                ("ResBlock(32→64) ×2", "8×8"),
                ("AdaptiveAvgPool2d(1)", "1×1"),
                ("Linear(64→10)", "10"),
            ],
        },
        _ => ModelInfo {
            name: "MLP (Multi-Layer Perceptron)",
            params: "~120 K",
            layers: vec![
                ("Linear(784→256)", "256"),
                ("ReLU", "256"),
                ("Linear(256→128)", "128"),
                ("ReLU", "128"),
                ("Linear(128→10)", "10"),
            ],
        },
    }
}

// ---------------------------------------------------------------------------
// FL code template generator
// ---------------------------------------------------------------------------

fn fl_code_template(
    num_clients: u32,
    data_dist: &str,
    num_rounds: u32,
    learning_rate: f64,
    batch_size: u32,
    model_preset: &str,
) -> String {
    let model_def = match model_preset {
        "cnn" => r#"class Net(nn.Module):
    """LeNet-style CNN for 28x28 grayscale images."""
    def __init__(self, input_dim, num_classes):
        super().__init__()
        self.input_dim = input_dim
        side = int(input_dim ** 0.5)
        self.side = side
        self.conv1 = nn.Conv2d(1, 32, 3, 1)
        self.conv2 = nn.Conv2d(32, 64, 3, 1)
        self.pool = nn.MaxPool2d(2)
        conv_out = ((side - 2) // 2 - 2) // 2
        self.fc1 = nn.Linear(64 * conv_out * conv_out, 128)
        self.fc2 = nn.Linear(128, num_classes)

    def forward(self, x):
        if x.dim() == 2:
            x = x.view(-1, 1, self.side, self.side)
        elif x.dim() == 3:
            x = x.unsqueeze(1)
        x = self.pool(torch.relu(self.conv1(x)))
        x = self.pool(torch.relu(self.conv2(x)))
        x = x.view(x.size(0), -1)
        x = torch.relu(self.fc1(x))
        return self.fc2(x)"#.to_string(),

        "resnet" => r#"class ResBlock(nn.Module):
    def __init__(self, in_ch, out_ch, stride=1):
        super().__init__()
        self.conv1 = nn.Conv2d(in_ch, out_ch, 3, stride, 1, bias=False)
        self.bn1 = nn.BatchNorm2d(out_ch)
        self.conv2 = nn.Conv2d(out_ch, out_ch, 3, 1, 1, bias=False)
        self.bn2 = nn.BatchNorm2d(out_ch)
        self.shortcut = nn.Sequential()
        if stride != 1 or in_ch != out_ch:
            self.shortcut = nn.Sequential(
                nn.Conv2d(in_ch, out_ch, 1, stride, bias=False),
                nn.BatchNorm2d(out_ch),
            )

    def forward(self, x):
        out = torch.relu(self.bn1(self.conv1(x)))
        out = self.bn2(self.conv2(out))
        return torch.relu(out + self.shortcut(x))

class Net(nn.Module):
    """Mini ResNet-18 for grayscale images."""
    def __init__(self, input_dim, num_classes):
        super().__init__()
        self.side = int(input_dim ** 0.5)
        self.prep = nn.Sequential(
            nn.Conv2d(1, 16, 3, 1, 1, bias=False), nn.BatchNorm2d(16), nn.ReLU()
        )
        self.layer1 = nn.Sequential(ResBlock(16, 16), ResBlock(16, 16))
        self.layer2 = nn.Sequential(ResBlock(16, 32, 2), ResBlock(32, 32))
        self.layer3 = nn.Sequential(ResBlock(32, 64, 2), ResBlock(64, 64))
        self.pool = nn.AdaptiveAvgPool2d(1)
        self.fc = nn.Linear(64, num_classes)

    def forward(self, x):
        if x.dim() == 2:
            x = x.view(-1, 1, self.side, self.side)
        elif x.dim() == 3:
            x = x.unsqueeze(1)
        x = self.prep(x)
        x = self.layer1(x)
        x = self.layer2(x)
        x = self.layer3(x)
        x = self.pool(x).view(x.size(0), -1)
        return self.fc(x)"#.to_string(),

        "custom" => r#"class Net(nn.Module):
    """Custom model — modify this architecture to suit your data."""
    def __init__(self, input_dim, num_classes):
        super().__init__()
        # TODO: Define your layers here
        self.flatten = nn.Flatten()
        self.fc1 = nn.Linear(input_dim, 128)
        self.fc2 = nn.Linear(128, num_classes)

    def forward(self, x):
        x = self.flatten(x)
        x = torch.relu(self.fc1(x))
        return self.fc2(x)"#.to_string(),

        _ /* mlp */ => r#"class Net(nn.Module):
    """Multi-Layer Perceptron for tabular / flattened image data."""
    def __init__(self, input_dim, num_classes):
        super().__init__()
        self.flatten = nn.Flatten()
        self.fc1 = nn.Linear(input_dim, 256)
        self.fc2 = nn.Linear(256, 128)
        self.fc3 = nn.Linear(128, num_classes)

    def forward(self, x):
        x = self.flatten(x)
        x = torch.relu(self.fc1(x))
        x = torch.relu(self.fc2(x))
        return self.fc3(x)"#.to_string(),
    };

    let partition_code = if data_dist == "non_iid" {
        format!(
            r#"
# --- Non-IID partitioning: sort by label, assign ~2 classes per client ---
def partition_data_non_iid(dataset, num_clients, num_classes_per_client=2):
    labels = dataset.tensors[1].numpy()
    sorted_indices = np.argsort(labels)
    num_classes = len(np.unique(labels))
    class_indices = []
    for c in range(num_classes):
        idx = sorted_indices[labels[sorted_indices] == c]
        class_indices.append(idx)
    client_indices = [[] for _ in range({num_clients})]
    classes_per_client = max(1, min(num_classes_per_client, num_classes))
    for i in range({num_clients}):
        chosen_classes = [(i * classes_per_client + j) % num_classes for j in range(classes_per_client)]
        for c in chosen_classes:
            shard_size = len(class_indices[c]) // {num_clients}
            start = (i * shard_size) % len(class_indices[c])
            end = start + shard_size
            if end <= len(class_indices[c]):
                client_indices[i].extend(class_indices[c][start:end].tolist())
            else:
                client_indices[i].extend(class_indices[c][start:].tolist())
                client_indices[i].extend(class_indices[c][:end - len(class_indices[c])].tolist())
    return [Subset(dataset, idx) for idx in client_indices]

client_datasets = partition_data_non_iid(train_dataset, {num_clients})
print(f"Data distribution: Non-IID (~2 classes per client)")
"#,
            num_clients = num_clients
        )
    } else {
        format!(
            r#"
# --- IID partitioning: random equal splits ---
def partition_data_iid(dataset, num_clients):
    indices = np.random.permutation(len(dataset)).tolist()
    shard_size = len(indices) // num_clients
    return [Subset(dataset, indices[i*shard_size:(i+1)*shard_size]) for i in range(num_clients)]

client_datasets = partition_data_iid(train_dataset, {num_clients})
print(f"Data distribution: IID (random equal splits)")
"#,
            num_clients = num_clients
        )
    };

    format!(
        r##"# ============================================================
# Federated Learning with PyTorch — FedAvg Simulation
# ============================================================
# Config: {num_clients} clients | {num_rounds} rounds | {data_dist_label} | LR={lr} | Batch={bs}
# Dataset is pre-loaded as `df` / `splits`
# ============================================================

import torch
import torch.nn as nn
import torch.optim as optim
from torch.utils.data import DataLoader, TensorDataset, Subset
import numpy as np
import pandas as pd
import copy
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt

# --- Prepare data from the pre-loaded DataFrame ---
print("Preparing data from dataset...")
print(f"  DataFrame shape: {{df.shape}}")
print(f"  Columns: {{df.columns.tolist()}}")

# Auto-detect features and labels
# Heuristic: if a column named 'label' or 'target' exists, use it; else last column
label_candidates = [c for c in df.columns if c.lower() in ("label", "target", "labels", "targets", "class")]
label_col = label_candidates[0] if label_candidates else df.columns[-1]
feature_cols = [c for c in df.columns if c != label_col]

# Handle array-valued columns (e.g. image pixels stored as lists/arrays)
sample_val = df[feature_cols[0]].iloc[0]
if isinstance(sample_val, dict) and "bytes" in sample_val:
    # HuggingFace image format: {{'bytes': b'...', 'path': '...'}}
    from PIL import Image
    import io
    def _load_img(val):
        img = Image.open(io.BytesIO(val["bytes"])).convert("L")
        return np.array(img, dtype=np.float32).flatten()
    arrays = [_load_img(v) for v in df[feature_cols[0]].values]
    X = torch.tensor(np.stack(arrays), dtype=torch.float32)
    print(f"  Decoded image bytes column: '{{feature_cols[0]}}' -> shape {{X.shape}}")
elif hasattr(sample_val, "__len__") and not isinstance(sample_val, str):
    # Each cell is an array-like (image pixels, embeddings, etc.)
    arrays = [np.array(v, dtype=np.float32) for v in df[feature_cols[0]].values]
    X = torch.tensor(np.stack(arrays), dtype=torch.float32)
    print(f"  Detected array-valued column: '{{feature_cols[0]}}' -> shape {{X.shape}}")
else:
    X = torch.tensor(df[feature_cols].values, dtype=torch.float32)

y = torch.tensor(df[label_col].values, dtype=torch.long)

# Normalize features to [0, 1] range
if X.max() > 1.0:
    X = X / X.max()

num_classes = len(torch.unique(y))
print(f"  Features shape: {{X.shape}} | Classes: {{num_classes}} | Samples: {{len(X)}}")

# Train/test split (80/20)
perm = torch.randperm(len(X))
split = int(0.8 * len(X))
train_dataset = TensorDataset(X[perm[:split]], y[perm[:split]])
test_dataset = TensorDataset(X[perm[split:]], y[perm[split:]])

# --- Model Definition ---
{model_def}
{partition_code}
for i, ds in enumerate(client_datasets):
    print(f"  Client {{i}}: {{len(ds)}} samples")

# --- Federated Learning Helpers ---
class FederatedClient:
    def __init__(self, client_id, dataset, model_template, lr, batch_size):
        self.id = client_id
        self.loader = DataLoader(dataset, batch_size=batch_size, shuffle=True, drop_last=False)
        self.model = copy.deepcopy(model_template)
        self.optimizer = optim.SGD(self.model.parameters(), lr=lr)
        self.criterion = nn.CrossEntropyLoss()

    def train_one_epoch(self):
        self.model.train()
        total_loss = 0.0
        n = 0
        for xb, yb in self.loader:
            self.optimizer.zero_grad()
            out = self.model(xb)
            loss = self.criterion(out, yb)
            loss.backward()
            self.optimizer.step()
            total_loss += loss.item() * len(xb)
            n += len(xb)
        return total_loss / max(n, 1)

def federated_average(global_model, client_models, client_sizes):
    """FedAvg: weighted average of client model parameters."""
    global_dict = global_model.state_dict()
    total = sum(client_sizes)
    for key in global_dict:
        global_dict[key] = sum(
            client_models[i].state_dict()[key].float() * (client_sizes[i] / total)
            for i in range(len(client_models))
        )
    global_model.load_state_dict(global_dict)

def evaluate(model, dataset):
    model.eval()
    loader = DataLoader(dataset, batch_size=256)
    correct, total = 0, 0
    with torch.no_grad():
        for xb, yb in loader:
            preds = model(xb).argmax(dim=1)
            correct += (preds == yb).sum().item()
            total += len(yb)
    return correct / max(total, 1) * 100.0

# --- Compute input dimensions ---
input_dim = X.shape[1] if X.dim() == 2 else int(np.prod(X.shape[1:]))

# --- Model Summary ---
global_model = Net(input_dim, num_classes)
total_params = sum(p.numel() for p in global_model.parameters())
trainable_params = sum(p.numel() for p in global_model.parameters() if p.requires_grad)
print()
print("=" * 50)
print(f"  Model: {{global_model.__class__.__name__}}")
print(f"  Total parameters: {{total_params:,}}")
print(f"  Trainable parameters: {{trainable_params:,}}")
print("=" * 50)
for name, module in global_model.named_children():
    params = sum(p.numel() for p in module.parameters())
    print(f"  {{name:20s}} | {{params:>8,}} params")
print("=" * 50)
print()

# --- Federated Training Loop ---
NUM_CLIENTS = {num_clients}
NUM_ROUNDS = {num_rounds}
LR = {lr}
BATCH_SIZE = {bs}

clients = [
    FederatedClient(i, client_datasets[i], global_model, LR, BATCH_SIZE)
    for i in range(NUM_CLIENTS)
]

round_losses = []
round_accuracies = []
client_accuracies_per_round = []

print(f"Starting Federated Training: {{NUM_ROUNDS}} rounds, {{NUM_CLIENTS}} clients")
print("-" * 50)

for rnd in range(1, NUM_ROUNDS + 1):
    # Distribute global model to all clients
    for client in clients:
        client.model.load_state_dict(global_model.state_dict())

    # Local training
    losses = []
    for client in clients:
        loss = client.train_one_epoch()
        losses.append(loss)

    # Aggregate
    client_models = [c.model for c in clients]
    client_sizes = [len(c.loader.dataset) for c in clients]
    federated_average(global_model, client_models, client_sizes)

    avg_loss = np.mean(losses)
    accuracy = evaluate(global_model, test_dataset)
    round_losses.append(avg_loss)
    round_accuracies.append(accuracy)

    # Per-client accuracy
    per_client_acc = []
    for client in clients:
        client.model.load_state_dict(global_model.state_dict())
        acc = evaluate(client.model, test_dataset)
        per_client_acc.append(acc)
    client_accuracies_per_round.append(per_client_acc)

    print(f"Round {{rnd:3d}}/{{NUM_ROUNDS}} | Loss: {{avg_loss:.4f}} | Accuracy: {{accuracy:.1f}}%")

print("-" * 50)
print(f"Final Global Accuracy: {{round_accuracies[-1]:.1f}}%")

# --- Visualization ---
fig, axes = plt.subplots(1, 3, figsize=(16, 4.5))

# 1) Training loss curve
axes[0].plot(range(1, NUM_ROUNDS + 1), round_losses, color="#10b981", linewidth=2)
axes[0].set_xlabel("Round")
axes[0].set_ylabel("Avg Loss")
axes[0].set_title("Training Loss per Round")
axes[0].grid(True, alpha=0.3)

# 2) Global accuracy curve
axes[1].plot(range(1, NUM_ROUNDS + 1), round_accuracies, color="#3b82f6", linewidth=2)
axes[1].set_xlabel("Round")
axes[1].set_ylabel("Accuracy (%)")
axes[1].set_title("Global Test Accuracy")
axes[1].grid(True, alpha=0.3)

# 3) Final per-client accuracy bar chart
final_client_accs = client_accuracies_per_round[-1]
colors = plt.cm.Set2(np.linspace(0, 1, NUM_CLIENTS))
bars = axes[2].bar(
    [f"C{{i}}" for i in range(NUM_CLIENTS)],
    final_client_accs,
    color=colors,
    edgecolor="#333",
)
axes[2].set_ylabel("Accuracy (%)")
axes[2].set_title("Per-Client Accuracy (Final Round)")
axes[2].grid(True, axis="y", alpha=0.3)
for bar, acc in zip(bars, final_client_accs):
    axes[2].text(bar.get_x() + bar.get_width()/2, bar.get_height() + 0.5,
                 f"{{acc:.1f}}%", ha="center", va="bottom", fontsize=8, color="#ccc")

plt.tight_layout()
plt.show()

summary = pd.DataFrame({{
    "Round": list(range(1, NUM_ROUNDS + 1)),
    "Avg Loss": [f"{{l:.4f}}" for l in round_losses],
    "Global Accuracy (%)": [f"{{a:.1f}}" for a in round_accuracies],
}})
print()
print("Training Summary:")
print(summary.to_string(index=False))

# ============================================================
# Explainable AI (XAI) Analysis
# ============================================================
print()
print("=" * 50)
print("  Explainable AI Analysis")
print("=" * 50)

# Helper: save plot to XAI tab (uses _xai_save_plot from runner)
def xai_plot():
    _xai_save_plot()

global _xai_html

# --- 1) Feature Importance (Gradient-based) ---
print("Computing gradient-based feature importance...")
try:
    global_model.eval()
    test_loader = DataLoader(test_dataset, batch_size=256)
    importance = torch.zeros(input_dim)
    total_samples = 0
    for xb, yb in test_loader:
        xb.requires_grad_(True)
        out = global_model(xb)
        loss_val = nn.CrossEntropyLoss()(out, yb)
        loss_val.backward()
        importance += xb.grad.abs().mean(dim=0).flatten().detach()
        total_samples += 1
        xb.requires_grad_(False)
    importance /= max(total_samples, 1)
    importance_np = importance.numpy()

    fig, ax = plt.subplots(figsize=(8, 5))
    fig.patch.set_facecolor("#111111")
    ax.set_facecolor("#111111")
    is_image = input_dim in [784, 1024, 3072]  # 28x28, 32x32, 32x32x3
    if is_image:
        side = int(input_dim ** 0.5)
        if side * side != input_dim:
            side = 28  # fallback for 784
        heatmap = importance_np[:side*side].reshape(side, side)
        im = ax.imshow(heatmap, cmap="hot", interpolation="bilinear")
        plt.colorbar(im, ax=ax, label="Avg |Gradient|")
        ax.set_title("Feature Importance (Saliency Map)", color="#fff", fontsize=14)
    else:
        top_k = min(20, len(importance_np))
        top_idx = np.argsort(importance_np)[-top_k:]
        ax.barh(range(top_k), importance_np[top_idx], color="#10b981", edgecolor="#333")
        feat_labels = [feature_cols[i] if i < len(feature_cols) else f"F{{i}}" for i in top_idx]
        ax.set_yticks(range(top_k))
        ax.set_yticklabels(feat_labels, color="#ccc", fontsize=9)
        ax.set_xlabel("Avg |Gradient|", color="#ccc")
        ax.set_title("Top Feature Importance (Gradient)", color="#fff", fontsize=14)
    ax.tick_params(colors="#888")
    plt.tight_layout()
    xai_plot()
    print("  ✓ Feature importance computed")
except Exception as e:
    print(f"  ⚠ Feature importance failed: {{e}}")

# --- 2) Confusion Matrix ---
print("Computing confusion matrix...")
try:
    from sklearn.metrics import confusion_matrix as sk_confusion_matrix
    global_model.eval()
    all_preds, all_labels = [], []
    with torch.no_grad():
        for xb, yb in DataLoader(test_dataset, batch_size=256):
            preds = global_model(xb).argmax(dim=1)
            all_preds.extend(preds.numpy())
            all_labels.extend(yb.numpy())
    cm = sk_confusion_matrix(all_labels, all_preds)

    fig, ax = plt.subplots(figsize=(7, 6))
    fig.patch.set_facecolor("#111111")
    ax.set_facecolor("#111111")
    try:
        import seaborn as sns
        sns.heatmap(cm, annot=True, fmt="d", cmap="Blues", ax=ax,
                    xticklabels=range(num_classes), yticklabels=range(num_classes),
                    cbar_kws={{"label": "Count"}})
    except ImportError:
        im = ax.imshow(cm, cmap="Blues", interpolation="nearest")
        plt.colorbar(im, ax=ax, label="Count")
        for i in range(cm.shape[0]):
            for j in range(cm.shape[1]):
                ax.text(j, i, str(cm[i, j]), ha="center", va="center",
                        color="white" if cm[i, j] > cm.max()/2 else "black", fontsize=8)
        ax.set_xticks(range(num_classes))
        ax.set_yticks(range(num_classes))
    ax.set_xlabel("Predicted", color="#ccc")
    ax.set_ylabel("Actual", color="#ccc")
    ax.set_title("Confusion Matrix", color="#fff", fontsize=14)
    ax.tick_params(colors="#888")
    plt.tight_layout()
    xai_plot()
    print("  ✓ Confusion matrix computed")
except Exception as e:
    print(f"  ⚠ Confusion matrix failed: {{e}}")

# --- 3) Per-Class Accuracy Breakdown ---
print("Computing per-class metrics...")
try:
    from sklearn.metrics import classification_report, precision_recall_fscore_support
    report = classification_report(all_labels, all_preds, output_dict=True, zero_division=0)
    prec, rec, f1, sup = precision_recall_fscore_support(all_labels, all_preds, zero_division=0)
    class_names = [str(c) for c in range(num_classes)]

    fig, ax = plt.subplots(figsize=(8, 5))
    fig.patch.set_facecolor("#111111")
    ax.set_facecolor("#111111")
    x_pos = np.arange(num_classes)
    w = 0.25
    ax.bar(x_pos - w, prec * 100, w, label="Precision", color="#3b82f6", edgecolor="#333")
    ax.bar(x_pos, rec * 100, w, label="Recall", color="#10b981", edgecolor="#333")
    ax.bar(x_pos + w, f1 * 100, w, label="F1-Score", color="#f59e0b", edgecolor="#333")
    ax.set_xticks(x_pos)
    ax.set_xticklabels(class_names, color="#ccc")
    ax.set_ylabel("Score (%)", color="#ccc")
    ax.set_title("Per-Class Precision / Recall / F1", color="#fff", fontsize=14)
    ax.legend(facecolor="#222", edgecolor="#444", labelcolor="#ccc")
    ax.tick_params(colors="#888")
    ax.grid(True, axis="y", alpha=0.2)
    plt.tight_layout()
    xai_plot()

    # Per-class metrics table -> _xai_html
    metrics_df = pd.DataFrame({{
        "Class": class_names,
        "Precision": [f"{{v:.3f}}" for v in prec],
        "Recall": [f"{{v:.3f}}" for v in rec],
        "F1-Score": [f"{{v:.3f}}" for v in f1],
        "Support": sup.astype(int),
    }})
    overall_acc = report.get("accuracy", 0)
    macro = report.get("macro avg", {{}})
    _xai_html = (
        f"<div style='margin-bottom:8px;color:#888;font-size:12px'>Overall Accuracy: {{overall_acc:.1%}} | "
        f"Macro F1: {{macro.get('f1-score', 0):.3f}}</div>"
        + metrics_df.to_html(classes="df-table", index=False)
    )
    print("  ✓ Per-class metrics computed")
except Exception as e:
    print(f"  ⚠ Per-class metrics failed: {{e}}")

# --- 4) SHAP Values ---
print("Computing SHAP values...")
try:
    import shap
    global_model.eval()
    background_size = min(100, len(test_dataset))
    bg_indices = np.random.choice(len(test_dataset), background_size, replace=False)
    bg_data = torch.stack([test_dataset[i][0] for i in bg_indices])
    sample_size = min(50, len(test_dataset))
    sample_indices = np.random.choice(len(test_dataset), sample_size, replace=False)
    sample_data = torch.stack([test_dataset[i][0] for i in sample_indices])

    # Wrap model to ensure flat 2D input (SHAP doesn't handle in-model reshapes well)
    class _FlatModel(nn.Module):
        def __init__(self, model, in_dim):
            super().__init__()
            self.model = model
            self.in_dim = in_dim
        def forward(self, x):
            if x.dim() == 2:
                return self.model(x)
            return self.model(x.view(x.size(0), -1))

    _flat = _FlatModel(global_model, input_dim)
    _flat.eval()
    bg_flat = bg_data.view(bg_data.size(0), -1).detach()
    sample_flat = sample_data.view(sample_data.size(0), -1).detach()

    explainer = shap.GradientExplainer(_flat, bg_flat)
    shap_values = explainer.shap_values(sample_flat)

    fig, ax = plt.subplots(figsize=(8, 5))
    fig.patch.set_facecolor("#111111")
    ax.set_facecolor("#111111")
    if isinstance(shap_values, list):
        sv = np.abs(np.array(shap_values)).mean(axis=0)
    else:
        sv = np.abs(shap_values)
    mean_shap = sv.mean(axis=0).flatten()
    top_k = min(20, len(mean_shap))
    top_idx = np.argsort(mean_shap)[-top_k:]
    feat_labels = [feature_cols[i] if i < len(feature_cols) else f"F{{i}}" for i in top_idx]
    ax.barh(range(top_k), mean_shap[top_idx], color="#8b5cf6", edgecolor="#333")
    ax.set_yticks(range(top_k))
    ax.set_yticklabels(feat_labels, color="#ccc", fontsize=9)
    ax.set_xlabel("Mean |SHAP|", color="#ccc")
    ax.set_title("SHAP Feature Importance", color="#fff", fontsize=14)
    ax.tick_params(colors="#888")
    plt.tight_layout()
    xai_plot()
    print("  ✓ SHAP values computed")
except ImportError:
    print("  ⚠ SHAP not installed — skipping. Install with: pip install shap")
except Exception as e:
    print(f"  ⚠ SHAP analysis failed: {{e}}")

# --- 5) Prediction Confidence Distribution ---
print("Computing prediction confidence distribution...")
try:
    global_model.eval()
    all_confs = []
    all_correct = []
    with torch.no_grad():
        for xb, yb in DataLoader(test_dataset, batch_size=256):
            probs = torch.softmax(global_model(xb), dim=1)
            max_conf, preds = probs.max(dim=1)
            all_confs.extend(max_conf.numpy())
            all_correct.extend((preds == yb).numpy())
    all_confs = np.array(all_confs)
    all_correct = np.array(all_correct)

    fig, ax = plt.subplots(figsize=(8, 5))
    fig.patch.set_facecolor("#111111")
    ax.set_facecolor("#111111")
    ax.hist(all_confs[all_correct], bins=30, alpha=0.7, label="Correct", color="#10b981", edgecolor="#333")
    ax.hist(all_confs[~all_correct], bins=30, alpha=0.7, label="Incorrect", color="#ef4444", edgecolor="#333")
    ax.set_xlabel("Max Prediction Probability", color="#ccc")
    ax.set_ylabel("Count", color="#ccc")
    ax.set_title("Prediction Confidence Distribution", color="#fff", fontsize=14)
    ax.legend(facecolor="#222", edgecolor="#444", labelcolor="#ccc")
    ax.tick_params(colors="#888")
    ax.grid(True, alpha=0.2)
    plt.tight_layout()
    xai_plot()
    print("  ✓ Prediction confidence computed")
except Exception as e:
    print(f"  ⚠ Prediction confidence failed: {{e}}")

# --- 6) t-SNE Visualization ---
print("Computing t-SNE embedding...")
try:
    from sklearn.manifold import TSNE
    global_model.eval()
    tsne_size = min(1000, len(test_dataset))
    tsne_indices = np.random.choice(len(test_dataset), tsne_size, replace=False)
    tsne_data = torch.stack([test_dataset[i][0] for i in tsne_indices])
    tsne_labels = np.array([test_dataset[i][1].item() for i in tsne_indices])

    # Extract penultimate layer activations via hook
    _activations = []
    def _hook_fn(module, inp, out):
        _activations.append(out.detach())

    # Find the penultimate layer
    children = list(global_model.named_children())
    if len(children) >= 2:
        hook_layer = getattr(global_model, children[-2][0])
    else:
        hook_layer = getattr(global_model, children[-1][0])
    handle = hook_layer.register_forward_hook(_hook_fn)

    with torch.no_grad():
        _ = global_model(tsne_data)
    handle.remove()

    if _activations:
        acts = _activations[0].numpy().reshape(tsne_size, -1)
    else:
        acts = tsne_data.numpy().reshape(tsne_size, -1)

    perplexity = min(30, tsne_size - 1)
    tsne = TSNE(n_components=2, random_state=42, perplexity=perplexity)
    coords = tsne.fit_transform(acts)

    fig, ax = plt.subplots(figsize=(8, 6))
    fig.patch.set_facecolor("#111111")
    ax.set_facecolor("#111111")
    scatter = ax.scatter(coords[:, 0], coords[:, 1], c=tsne_labels, cmap="tab10",
                         s=8, alpha=0.7, edgecolors="none")
    cbar = plt.colorbar(scatter, ax=ax, label="Class")
    cbar.ax.yaxis.label.set_color("#ccc")
    cbar.ax.tick_params(colors="#888")
    ax.set_title("t-SNE of Learned Representations", color="#fff", fontsize=14)
    ax.set_xlabel("t-SNE 1", color="#ccc")
    ax.set_ylabel("t-SNE 2", color="#ccc")
    ax.tick_params(colors="#888")
    plt.tight_layout()
    xai_plot()
    print("  ✓ t-SNE computed")
except ImportError:
    print("  ⚠ sklearn not installed — skipping t-SNE. Install with: pip install scikit-learn")
except Exception as e:
    print(f"  ⚠ t-SNE visualization failed: {{e}}")

print()
print("=" * 50)
print("  XAI Analysis Complete")
print("=" * 50)
"##,
        num_clients = num_clients,
        num_rounds = num_rounds,
        data_dist_label = if data_dist == "non_iid" {
            "Non-IID"
        } else {
            "IID"
        },
        lr = learning_rate,
        bs = batch_size,
        model_def = model_def,
        partition_code = partition_code,
    )
}

// ---------------------------------------------------------------------------
// Workspace Editor (main component)
// ---------------------------------------------------------------------------

#[component]
fn WorkspaceEditor(workspace: crate::models::Workspace, id: String) -> Element {
    let ws_name = workspace.name.clone();
    let ds_name = workspace.dataset_name.clone();

    let initial_code = if workspace.code.is_empty() {
        format!(
            r#"# Dataset '{}' is pre-loaded as `df`
# Available splits: splits.keys()
import pandas as pd
import numpy as np

print("Dataset shape:", df.shape)
print("Columns:", df.columns.tolist())

# Create a summary table (visible in the Table tab)
summary = df.describe()
print(summary)
"#,
            workspace.dataset_name
        )
    } else {
        workspace.code.clone()
    };
    let mut code = use_signal(move || initial_code.clone());
    let mut is_running = use_signal(|| false);
    let mut is_saving = use_signal(|| false);
    let mut result = use_signal(|| Option::<ExecutionResult>::None);
    let mut active_tab = use_signal(|| "console".to_string());

    // Load the last saved run result
    let ws_id_for_result = id.clone();
    let saved_result = use_resource(move || {
        let wid = ws_id_for_result.clone();
        async move { get_last_run_result_server(wid).await }
    });
    // Seed result signal from saved data (runs once when loaded)
    use_effect(move || {
        if let Some(Ok(Some(saved))) = saved_result()
            && result().is_none()
        {
            result.set(Some(saved));
        }
    });

    // FL configuration state
    let mut show_fl_panel = use_signal(|| false);
    let fl_num_clients = use_signal(|| 3u32);
    let fl_data_dist = use_signal(|| "iid".to_string());
    let fl_num_rounds = use_signal(|| 5u32);
    let fl_learning_rate = use_signal(|| "0.01".to_string());
    let fl_batch_size = use_signal(|| 32u32);
    let fl_model_preset = use_signal(|| "mlp".to_string());

    let nav = use_navigator();

    let id_run = id.clone();
    let handle_run = move |_| {
        let ws_id = id_run.clone();
        async move {
            // Sync CodeMirror value to signal before running
            let _ = document::eval(
                r#"if (window._cmEditor) {
                    var ta = document.getElementById('code-editor-textarea');
                    if (ta) {
                        ta.value = window._cmEditor.getValue();
                        ta.dispatchEvent(new Event('input', { bubbles: true }));
                    }
                }
                return true;"#,
            )
            .await;
            let current_code = code();
            is_running.set(true);
            let _ = save_workspace_code_server(ws_id.clone(), current_code.clone()).await;
            match run_python_server(ws_id, current_code).await {
                Ok(exec_result) => result.set(Some(exec_result)),
                Err(e) => result.set(Some(ExecutionResult {
                    stdout: String::new(),
                    stderr: format!("Server error: {e}"),
                    plots: Vec::new(),
                    table_html: None,
                    xai_plots: Vec::new(),
                    xai_html: None,
                })),
            }
            is_running.set(false);
        }
    };

    let id_save = id.clone();
    let handle_save = move |_| {
        let ws_id = id_save.clone();
        async move {
            is_saving.set(true);
            // Sync CodeMirror value to signal before saving
            let _ = document::eval(
                r#"if (window._cmEditor) {
                    var ta = document.getElementById('code-editor-textarea');
                    if (ta) {
                        ta.value = window._cmEditor.getValue();
                        ta.dispatchEvent(new Event('input', { bubbles: true }));
                    }
                }
                return true;"#,
            )
            .await;
            let _ = save_workspace_code_server(ws_id, code()).await;
            is_saving.set(false);
        }
    };

    let handle_delete = move |_| {
        let ws_id = id.clone();
        async move {
            if delete_workspace_server(ws_id).await.is_ok() {
                nav.push(Route::Workspaces {});
            }
        }
    };

    let editor_class = if show_fl_panel() {
        "workspace-editor-container with-sidebar"
    } else {
        "workspace-editor-container"
    };

    rsx! {
        // Back link
        Link {
            to: Route::Workspaces {},
            class: "text-[#888] hover:text-white text-sm mb-4 inline-block transition-colors",
            "← Back to Workspaces"
        }

        // Header
        div { class: "flex items-center justify-between mb-6",
            div {
                h1 { class: "text-2xl font-bold text-white", "{ws_name}" }
                div { class: "flex items-center gap-2 mt-1",
                    span { class: "text-xs border border-[#2a2a2a] text-[#888] px-2 py-1 rounded-full",
                        "📊 {ds_name}"
                    }
                }
            }
            button {
                class: "text-[#ff3333] border border-[#ff3333] hover:bg-[#ff3333] hover:text-black px-3 py-1 rounded-lg text-sm transition-colors",
                onclick: handle_delete,
                "Delete"
            }
        }

        // Editor + FL sidebar layout
        div { class: "workspace-editor-layout mb-4",
            // Editor section
            div { class: "workspace-editor-main",
                div { class: "{editor_class}",
                    div { class: "editor-header",
                        span { class: "text-sm text-[#888]", "Python Editor" }
                        div { class: "flex gap-2 items-center",
                            button {
                                class: if show_fl_panel() { "fl-config-toggle active" } else { "fl-config-toggle" },
                                onclick: move |_| show_fl_panel.set(!show_fl_panel()),
                                "FL Config"
                            }
                            button {
                                class: "text-sm border border-[#444] text-[#888] hover:text-white hover:border-[#666] px-3 py-1 rounded transition-colors disabled:opacity-50",
                                disabled: is_saving(),
                                onclick: handle_save,
                                if is_saving() {
                                    "Saving..."
                                } else {
                                    "Save"
                                }
                            }
                            button {
                                class: "text-sm bg-emerald-600 hover:bg-emerald-500 text-white px-4 py-1 rounded transition-colors disabled:opacity-50 flex items-center gap-1",
                                disabled: is_running(),
                                onclick: handle_run,
                                if is_running() {
                                    "⟳ Running..."
                                } else {
                                    "▶ Run"
                                }
                            }
                        }
                    }

                    div { id: "code-editor-wrapper",
                        textarea {
                            id: "code-editor-textarea",
                            class: "code-textarea",
                            spellcheck: "false",
                            value: "{code}",
                            oninput: move |e| code.set(e.value()),
                        }
                    }
                }
            }

            // FL Configuration Sidebar
            if show_fl_panel() {
                FLConfigPanel {
                    code,
                    num_clients: fl_num_clients,
                    data_dist: fl_data_dist,
                    num_rounds: fl_num_rounds,
                    learning_rate: fl_learning_rate,
                    batch_size: fl_batch_size,
                    model_preset: fl_model_preset,
                    on_close: move |_| show_fl_panel.set(false),
                }
            }
        }

        // Initialize CodeMirror after mount
        CodeMirrorInit { code }

        // Output section
        div { class: "workspace-output-container",
            div { class: "output-tabs",
                button {
                    class: if active_tab() == "console" { "output-tab active" } else { "output-tab" },
                    onclick: move |_| active_tab.set("console".to_string()),
                    "Console"
                }
                button {
                    class: if active_tab() == "plots" { "output-tab active" } else { "output-tab" },
                    onclick: move |_| active_tab.set("plots".to_string()),
                    "Plots"
                    if let Some(ref res) = result() {
                        if !res.plots.is_empty() {
                            span { class: "tab-badge", "{res.plots.len()}" }
                        }
                    }
                }
                button {
                    class: if active_tab() == "table" { "output-tab active" } else { "output-tab" },
                    onclick: move |_| active_tab.set("table".to_string()),
                    "Table"
                    if let Some(ref res) = result() {
                        if res.table_html.is_some() {
                            span { class: "tab-badge", "1" }
                        }
                    }
                }
                button {
                    class: if active_tab() == "xai" { "output-tab active" } else { "output-tab" },
                    onclick: move |_| active_tab.set("xai".to_string()),
                    "XAI"
                    if let Some(ref res) = result() {
                        if !res.xai_plots.is_empty() || res.xai_html.is_some() {
                            {
                                let count = res.xai_plots.len() + if res.xai_html.is_some() { 1 } else { 0 };
                                rsx! {
                                    span { class: "tab-badge", "{count}" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "output-content",
                OutputPanel {
                    is_running: is_running(),
                    result: result(),
                    active_tab: active_tab(),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// FL Configuration Panel (sidebar)
// ---------------------------------------------------------------------------

#[component]
fn FLConfigPanel(
    mut code: Signal<String>,
    mut num_clients: Signal<u32>,
    mut data_dist: Signal<String>,
    mut num_rounds: Signal<u32>,
    mut learning_rate: Signal<String>,
    mut batch_size: Signal<u32>,
    mut model_preset: Signal<String>,
    on_close: EventHandler,
) -> Element {
    let model_info = use_memo(move || {
        let preset = model_preset();
        let info = get_model_info(&preset);
        (
            info.name.to_string(),
            info.params.to_string(),
            info.layers
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect::<Vec<_>>(),
        )
    });

    let handle_generate = move |_| {
        let lr_val: f64 = learning_rate().parse().unwrap_or(0.01);
        let generated = fl_code_template(
            num_clients(),
            &data_dist(),
            num_rounds(),
            lr_val,
            batch_size(),
            &model_preset(),
        );
        code.set(generated.clone());

        // Sync CodeMirror editor and refresh to reapply styles
        let escaped = generated
            .replace('\\', "\\\\")
            .replace('`', "\\`")
            .replace("${", "\\${");
        let js = format!(
            r#"if (window._cmEditor) {{
                window._cmEditor.setValue(`{}`);
                window._cmEditor.refresh();
                setTimeout(function() {{ window._cmEditor.refresh(); }}, 50);
            }}"#,
            escaped
        );
        document::eval(&js);
    };

    rsx! {
        div { class: "fl-sidebar",
            // Header
            div { class: "fl-sidebar-header",
                h3 { "Federated Learning" }
                button {
                    class: "fl-sidebar-close",
                    onclick: move |_| on_close.call(()),
                    "✕"
                }
            }

            div { class: "fl-sidebar-body",
                // Number of Clients
                div { class: "fl-section",
                    label { class: "fl-label", "Clients" }
                    div { class: "fl-stepper",
                        button {
                            onclick: move |_| {
                                let v = num_clients();
                                if v > 2 {
                                    num_clients.set(v - 1);
                                }
                            },
                            "−"
                        }
                        input {
                            r#type: "number",
                            value: "{num_clients}",
                            min: "2",
                            max: "20",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    num_clients.set(v.clamp(2, 20));
                                }
                            },
                        }
                        button {
                            onclick: move |_| {
                                let v = num_clients();
                                if v < 20 {
                                    num_clients.set(v + 1);
                                }
                            },
                            "+"
                        }
                    }
                }

                // Data Distribution
                div { class: "fl-section",
                    label { class: "fl-label", "Data Distribution" }
                    div { class: "fl-toggle-group",
                        button {
                            class: if data_dist() == "iid" { "fl-toggle-btn active" } else { "fl-toggle-btn" },
                            onclick: move |_| data_dist.set("iid".to_string()),
                            "IID"
                        }
                        button {
                            class: if data_dist() == "non_iid" { "fl-toggle-btn active" } else { "fl-toggle-btn" },
                            onclick: move |_| data_dist.set("non_iid".to_string()),
                            "Non-IID"
                        }
                    }
                    p { class: "fl-hint",
                        if data_dist() == "iid" {
                            "Random equal splits across clients"
                        } else {
                            "Each client gets ~2 classes"
                        }
                    }
                }

                // Training Rounds
                div { class: "fl-section",
                    label { class: "fl-label", "Rounds" }
                    div { class: "fl-stepper",
                        button {
                            onclick: move |_| {
                                let v = num_rounds();
                                if v > 1 {
                                    num_rounds.set(v - 1);
                                }
                            },
                            "−"
                        }
                        input {
                            r#type: "number",
                            value: "{num_rounds}",
                            min: "1",
                            max: "100",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    num_rounds.set(v.clamp(1, 100));
                                }
                            },
                        }
                        button {
                            onclick: move |_| {
                                let v = num_rounds();
                                if v < 100 {
                                    num_rounds.set(v + 1);
                                }
                            },
                            "+"
                        }
                    }
                }

                // Model Selection
                div { class: "fl-section",
                    label { class: "fl-label", "Model Architecture" }
                    select {
                        class: "fl-select",
                        value: "{model_preset}",
                        onchange: move |e| model_preset.set(e.value()),
                        option { value: "mlp", "MLP — Multi-Layer Perceptron" }
                        option { value: "cnn", "CNN — Convolutional (LeNet)" }
                        option { value: "resnet", "ResNet-18 (Mini)" }
                        option { value: "custom", "Custom — Edit in Editor" }
                    }
                }

                div { class: "fl-divider" }

                // Learning Rate & Batch Size (compact row)
                div { class: "fl-section",
                    label { class: "fl-label", "Learning Rate" }
                    input {
                        class: "fl-input",
                        r#type: "number",
                        step: "0.001",
                        min: "0.0001",
                        max: "1.0",
                        value: "{learning_rate}",
                        oninput: move |e| learning_rate.set(e.value()),
                    }
                }

                div { class: "fl-section",
                    label { class: "fl-label", "Batch Size" }
                    div { class: "fl-stepper",
                        button {
                            onclick: move |_| {
                                let v = batch_size();
                                if v > 8 {
                                    batch_size.set(v / 2);
                                }
                            },
                            "÷"
                        }
                        input {
                            r#type: "number",
                            value: "{batch_size}",
                            min: "1",
                            max: "512",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<u32>() {
                                    batch_size.set(v.clamp(1, 512));
                                }
                            },
                        }
                        button {
                            onclick: move |_| {
                                let v = batch_size();
                                if v < 512 {
                                    batch_size.set(v * 2);
                                }
                            },
                            "×"
                        }
                    }
                }

                div { class: "fl-divider" }

                // Model Summary
                {
                    let (name, params, layers) = &*model_info.read();
                    rsx! {
                        div { class: "fl-section",
                            label { class: "fl-label", "Model Summary" }
                            div { class: "fl-model-summary",
                                div { class: "fl-model-summary-header",
                                    span { class: "fl-model-name", "{name}" }
                                    span { class: "fl-model-params", "{params} params" }
                                }
                                div { class: "fl-model-layers",
                                    for (layer_name , shape) in layers.iter() {
                                        div { class: "fl-model-layer",
                                            span { class: "fl-model-layer-name", "{layer_name}" }
                                            span { class: "fl-model-layer-shape", "→ {shape}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Generate button
                button { class: "fl-generate-btn", onclick: handle_generate, "Generate FL Code" }
            }
        }
    }
}

#[component]
fn OutputPanel(is_running: bool, result: Option<ExecutionResult>, active_tab: String) -> Element {
    if is_running {
        return rsx! {
            div { class: "text-center py-8",
                div { class: "text-2xl animate-spin mb-2", "⟳" }
                p { class: "text-[#888]", "Executing Python code..." }
            }
        };
    }

    let Some(res) = result else {
        return rsx! {
            div { class: "text-center py-8",
                p { class: "text-[#555]", "Click \"▶ Run\" to execute your code" }
            }
        };
    };

    match active_tab.as_str() {
        "console" => rsx! {
            div { class: "console-output",
                if !res.stdout.is_empty() {
                    pre { class: "stdout", "{res.stdout}" }
                }
                if !res.stderr.is_empty() {
                    pre { class: "stderr", "{res.stderr}" }
                }
                if res.stdout.is_empty() && res.stderr.is_empty() {
                    p { class: "text-[#555] text-sm italic", "No console output" }
                }
            }
        },
        "plots" => rsx! {
            div { class: "plots-output",
                if res.plots.is_empty() {
                    p { class: "text-[#555] text-sm italic py-4",
                        "No plots generated. Use plt.show() to display plots."
                    }
                } else {
                    for (i , plot) in res.plots.iter().enumerate() {
                        div { class: "plot-container",
                            img {
                                src: "data:image/png;base64,{plot}",
                                alt: "Plot {i}",
                                class: "plot-image",
                            }
                        }
                    }
                }
            }
        },
        "table" => rsx! {
            div { class: "table-output",
                if let Some(ref html) = res.table_html {
                    div { dangerous_inner_html: "{html}" }
                } else {
                    p { class: "text-[#555] text-sm italic py-4",
                        "No table output. Assign a DataFrame to a variable to see it here."
                    }
                }
            }
        },
        "xai" => rsx! {
            div { class: "xai-output",
                if res.xai_plots.is_empty() && res.xai_html.is_none() {
                    div { class: "text-center py-8",
                        p { class: "text-[#555] text-sm italic",
                            "No XAI output. Use the FL Config panel to generate code with Explainable AI analysis."
                        }
                    }
                } else {
                    if !res.xai_plots.is_empty() {
                        div { class: "xai-plots-grid",
                            for (i , plot) in res.xai_plots.iter().enumerate() {
                                div { class: "xai-plot-card",
                                    img {
                                        src: "data:image/png;base64,{plot}",
                                        alt: "XAI Plot {i}",
                                        class: "xai-plot-image",
                                    }
                                }
                            }
                        }
                    }
                    if let Some(ref html) = res.xai_html {
                        div { class: "xai-table-section",
                            h3 { class: "xai-section-title", "Per-Class Metrics" }
                            div { class: "table-output",
                                div { dangerous_inner_html: "{html}" }
                            }
                        }
                    }
                }
            }
        },
        _ => rsx! {},
    }
}

#[component]
fn CodeMirrorInit(mut code: Signal<String>) -> Element {
    use_effect(move || {
        spawn(async move {
            // Destroy any previous CodeMirror instance so we re-init on SPA navigation
            let _ = document::eval(
                r#"if (window._cmEditor) {
                    window._cmEditor.toTextArea();
                    window._cmEditor = null;
                }
                return true;"#,
            )
            .await;

            // Poll until CodeMirror CDN script is loaded and textarea exists,
            // then dynamically load the Python mode to avoid race conditions
            let ready = document::eval(
                r#"async function waitForCM() {
                    for (let i = 0; i < 50; i++) {
                        if (typeof CodeMirror !== 'undefined' && document.getElementById('code-editor-textarea')) {
                            // Dynamically load Python mode after main lib is ready
                            if (!CodeMirror.modes.python) {
                                await new Promise((resolve, reject) => {
                                    var s = document.createElement('script');
                                    s.src = 'https://cdnjs.cloudflare.com/ajax/libs/codemirror/5.65.18/mode/python/python.min.js';
                                    s.onload = resolve;
                                    s.onerror = reject;
                                    document.head.appendChild(s);
                                });
                            }
                            return true;
                        }
                        await new Promise(r => setTimeout(r, 100));
                    }
                    return false;
                }
                return await waitForCM();"#,
            );
            let is_ready = ready.await;
            if is_ready.is_err() {
                return;
            }

            let init_code = code();
            let escaped = init_code
                .replace('\\', "\\\\")
                .replace('`', "\\`")
                .replace("${", "\\${");

            let js = format!(
                r#"
                (function() {{
                    if (window._cmEditor) return;
                    var ta = document.getElementById('code-editor-textarea');
                    if (!ta || typeof CodeMirror === 'undefined') return;

                    var editor = CodeMirror.fromTextArea(ta, {{
                        mode: 'python',
                        theme: 'material-darker',
                        lineNumbers: true,
                        indentUnit: 4,
                        tabSize: 4,
                        indentWithTabs: false,
                        lineWrapping: true,
                        matchBrackets: true,
                        autoCloseBrackets: true,
                        extraKeys: {{
                            "Tab": function(cm) {{
                                cm.replaceSelection("    ", "end");
                            }}
                        }}
                    }});
                    editor.setValue(`{escaped}`);
                    editor.setSize(null, '400px');
                    window._cmEditor = editor;

                    editor.on('change', function() {{
                        ta.value = editor.getValue();
                        ta.dispatchEvent(new Event('input', {{ bubbles: true }}));
                    }});
                }})();
                "#
            );

            let _ = document::eval(&js);
        });
    });

    rsx! {}
}
