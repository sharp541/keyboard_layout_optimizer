# %%
from datasets import load_dataset
from huggingface_hub import login
import pykakasi
import re
import json
import os


# %%
if not os.path.exists("./.env/token.json"):
    login()
else:
    with open("./.env/token.json", "r", encoding="utf-8") as f:
        config = json.load(f)
        access_key = config["access_key"]
        os.environ["CURL_CA_BUNDLE"] = config["ca_bundle"]
        login(access_key)
# %%
dataset = load_dataset("bigcode/starcoderdata",
                       data_dir="python", split="train", streaming=True)
# %%
ja_dataset= load_dataset("izumi-lab/cc100-ja-filter-ja-normal", split="train", streaming=True, ignore_verifications=True)

# %%
qwerty_layout = set([
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
    'l', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ' '
])

def clean(text):
    t = re.sub(r"\*+", "*", "".join(c if c in qwerty_layout else "" for c in text.lower())).strip()
    # remove multiple spaces
    t = re.sub(r"\s+", " ", t)
    return t

def sample_dataset(dataset, selector , max_size=1 * 1024 * 1024):
    sampled_files = []
    total_size = 0
    for entry in dataset:
        content = selector(entry)
        if not content:
            raise ValueError("selector returned empty content")
        size = len(content.encode("utf-8"))
        if total_size + size > max_size:
            break
        sampled_files.append(content)
        total_size += size
    return sampled_files
# %%
en = "../data/en.txt"
with open(en, "w", encoding="utf-8") as f:
    selector = lambda x: x["content"] if "content" in x else None
    sampled_files = sample_dataset(dataset, max_size=1 * 1024 * 1024)
    text = "".join(sampled_files)
    cleaned = clean(text)
    f.write(f"{cleaned}\n")

# %%
ja_raw = "../data/ja_raw.txt"
ja = "../data/ja.txt"
# %%
with open(ja_raw, "w", encoding="utf-8") as f:
    selector = lambda x: x["text"] if "text" in x else None
    sampled_files = sample_dataset(ja_dataset, selector, max_size=2 * 1024 * 1024)
    text = "".join(sampled_files)
    f.write(f"{text}")

# %%
with open(ja_raw, "r", encoding="utf-8") as f:
    text = f.read().strip()
# %%
kks = pykakasi.kakasi()
lines = text.split("。")
ret = ""
for line in lines:
    if not line.strip():
        continue
    result = kks.convert(line)
    ret += "".join([item["kunrei"] for item in result])
    ret += " "
# %%
with open(ja, "w", encoding="utf-8") as f:
    f.write(f"{clean(ret)}")

# %%
