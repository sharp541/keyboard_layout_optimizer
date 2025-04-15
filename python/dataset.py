# %%
from datasets import load_dataset
from huggingface_hub import login
import re

# %%
login("")
dataset = load_dataset("bigcode/the-stack",
                       data_dir="data/python", split="train", streaming=True)


# %%
qwerty_layout = set([
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
    'l', 'z', 'x', 'c', 'v', 'b', 'n', 'm',
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


def clean(text):
    return re.sub(r"\*+", "*", "".join(c if c in qwerty_layout else "" for c in text.lower())).strip()


with open("en.txt", "w", encoding="utf-8") as f:
    text = "".join(sampled_files)
    cleaned = clean(text)
    f.write(f"{cleaned}\n")

# %%
with open("ja.txt", "r", encoding="utf-8") as f:
    text = f.read()

with open("ja.txt", "w", encoding="utf-8") as f:
    f.write(clean(text))

# %%
