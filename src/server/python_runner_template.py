import sys, os, json, io, traceback

# Setup
os.chdir(r"__ABS_BASE_STR__")
_stdout_capture = io.StringIO()
_stderr_capture = io.StringIO()
_plots = []
_table_html = None
_xai_plots = []
_xai_html = None

def _xai_save_plot():
    """Save current matplotlib figure to _xai_plots (base64 PNG)."""
    import base64, matplotlib.pyplot as plt
    buf = io.BytesIO()
    plt.savefig(buf, format="png", bbox_inches="tight", dpi=100, facecolor="#111111")
    buf.seek(0)
    _xai_plots.append(base64.b64encode(buf.read()).decode("utf-8"))
    plt.close("all")

# Load dataset
try:
    import pandas as pd
    import numpy as np
except ImportError as e:
    print(json.dumps({"stdout": "", "stderr": f"Missing required library: {e}\nInstall with: pip install pandas numpy matplotlib", "plots": [], "table_html": None}))
    sys.exit(0)

parquet_files = __PARQUET_PATHS_JSON__

df = pd.DataFrame()
splits = {}

for pf in parquet_files:
    try:
        _df = pd.read_parquet(pf)
        name = os.path.splitext(os.path.basename(pf))[0]
        # Clean split name: remove "train-00000-of-00001" -> "train"
        parts = name.split("-")
        split_name = parts[0] if parts[0] in ("train", "test", "validation", "val") else name
        splits[split_name] = _df
    except Exception:
        pass

if splits:
    # Pick the first available split as the default df
    for preferred in ("train", "test", "validation", "val"):
        if preferred in splits:
            df = splits[preferred]
            break
    if df.empty and splits:
        df = next(iter(splits.values()))

if df.empty:
    raise RuntimeError(
        f"No data loaded: DataFrame is empty. "
        f"Searched {len(parquet_files)} parquet path(s): {parquet_files}. "
        f"Check that the dataset files exist and are valid parquet files."
    )

# Patch matplotlib
try:
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt

    _original_show = plt.show
    def _patched_show(*args, **kwargs):
        import base64
        buf = io.BytesIO()
        plt.savefig(buf, format="png", bbox_inches="tight", dpi=100, facecolor="#111111")
        buf.seek(0)
        _plots.append(base64.b64encode(buf.read()).decode("utf-8"))
        plt.close("all")
    plt.show = _patched_show
except ImportError:
    pass

# Redirect stdout
_old_stdout = sys.stdout
_old_stderr = sys.stderr
sys.stdout = _stdout_capture
sys.stderr = _stderr_capture

# Execute user code
try:
    exec(compile('''__USER_CODE__''', '<workspace>', 'exec'))
except Exception:
    traceback.print_exc()

# Check if the last expression produced a DataFrame for table display
sys.stdout = _old_stdout
sys.stderr = _old_stderr

# Try to capture any DataFrame that was assigned or displayed
for _vname in reversed(list(dir())):
    if _vname.startswith('_'):
        continue
    _vobj = locals().get(_vname)
    if _vobj is not None and isinstance(_vobj, pd.DataFrame) and _vobj is not df:
        try:
            _table_html = _vobj.head(100).to_html(classes="df-table", index=True, max_rows=100, max_cols=50)
        except Exception:
            pass
        break

result = {
    "stdout": _stdout_capture.getvalue(),
    "stderr": _stderr_capture.getvalue(),
    "plots": _plots,
    "table_html": _table_html,
    "xai_plots": _xai_plots,
    "xai_html": _xai_html,
}
print("__FEDLAB_RESULT__" + json.dumps(result))
