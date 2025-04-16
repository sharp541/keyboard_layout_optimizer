# %%
from datasets import load_dataset
from huggingface_hub import login
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
dataset = load_dataset("bigcode/starcoderdata",
                       data_dir="python", split="train", streaming=True)


# %%
qwerty_layout = set([
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
    'l', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ' '
])
# %%
sampled_files = []
total_size = 0
max_size = 1 * 1024 * 1024  # 1MB

for entry in dataset:
    content = entry["content"]
    size = len(content.encode("utf-8"))
    if total_size + size > max_size:
        break
    sampled_files.append(content)
    total_size += size

# %%
def clean(text):
    t = re.sub(r"\*+", "*", "".join(c if c in qwerty_layout else "" for c in text.lower())).strip()
    # remove multiple spaces
    t = re.sub(r"\s+", " ", t)
    return t

en = "../data/en.txt"
with open(en, "w", encoding="utf-8") as f:
    text = "".join(sampled_files)
    cleaned = clean(text)
    f.write(f"{cleaned}\n")

# %%
ja = "../data/ja.txt"
with open(ja, "r", encoding="utf-8") as f:
    text = f.read()

with open(ja, "w", encoding="utf-8") as f:
    f.write(clean(text))

# %%
