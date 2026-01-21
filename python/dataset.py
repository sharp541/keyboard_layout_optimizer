"""
データセットの取得と加工を行うスクリプト

使用方法:
    python dataset.py init      # 生データの取得と保存
    python dataset.py           # 加工ファイルの作成
"""
import argparse
from datasets import load_dataset
from huggingface_hub import login
import pykakasi
import re
import json
import os
from pathlib import Path


qwerty_layout = set([
    'q', 'w', 'e', 'r', 't', 'y', 'u', 'i', 'o', 'p', 'a', 's', 'd', 'f', 'g', 'h', 'j', 'k',
    'l', 'z', 'x', 'c', 'v', 'b', 'n', 'm', ' ', '.', ','
])


def clean(text):
    """
    テキストをクリーンアップする

    Args:
        text: クリーンアップするテキスト

    Returns:
        クリーンアップされたテキスト
    """
    t = re.sub(
        r"\*+", "*", "".join(c if c in qwerty_layout else "" for c in text.lower())).strip()
    # remove multiple spaces
    t = re.sub(r"\s+", " ", t)
    return t


def sample_dataset(dataset, selector, max_size=1 * 1024 * 1024):
    """
    データセットから指定サイズまでサンプリングする

    Args:
        dataset: サンプリングするデータセット
        selector: エントリからコンテンツを抽出する関数
        max_size: 最大サイズ（バイト）

    Returns:
        サンプリングされたファイルのリスト
    """
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


def login_huggingface():
    """
    HuggingFaceにログインする
    """
    if not os.path.exists("./.env/token.json"):
        login()
    else:
        with open("./.env/token.json", "r", encoding="utf-8") as f:
            config = json.load(f)
            access_key = config["access_key"]
        login(access_key)


def init_command(data_dir="../data"):
    """
    initサブコマンド: 生データの取得と保存

    Args:
        data_dir: データを保存するディレクトリ
    """
    print("HuggingFaceにログイン中...")
    login_huggingface()

    data_path = Path(data_dir)
    data_path.mkdir(parents=True, exist_ok=True)

    print("英語データセットを取得中...")
    dataset = load_dataset("bigcode/starcoderdata",
                           data_dir="python", split="train", streaming=True)
    en_raw_path = data_path / "en_raw.txt"
    with open(en_raw_path, "w", encoding="utf-8") as f:
        def selector(x): return x["content"] if "content" in x else None
        sampled_files = sample_dataset(
            dataset, selector, max_size=4 * 1024 * 1024)
        text = "".join(sampled_files)
        f.write(f"{text}")
    print(f"英語生データを保存しました: {en_raw_path}")

    print("日本語データセットを取得中...")
    ja_dataset = load_dataset(
        "izumi-lab/cc100-ja-filter-ja-normal", split="train", streaming=True)
    ja_raw_path = data_path / "ja_raw.txt"
    with open(ja_raw_path, "w", encoding="utf-8") as f:
        def selector(x): return x["text"] if "text" in x else None
        sampled_files = sample_dataset(
            ja_dataset, selector, max_size=4 * 1024 * 1024)
        text = "".join(sampled_files)
        f.write(f"{text}")
    print(f"日本語生データを保存しました: {ja_raw_path}")
    print("生データの取得が完了しました。")


def process_command(data_dir="../data"):
    """
    サブコマンド無し: 加工ファイルの作成

    Args:
        data_dir: データが保存されているディレクトリ
    """
    data_path = Path(data_dir)

    # 英語データの加工
    en_raw_path = data_path / "en_raw.txt"
    en_path = data_path / "en.txt"

    if not en_raw_path.exists():
        print(
            f"エラー: {en_raw_path} が見つかりません。先に 'python dataset.py init' を実行してください。")
        return

    print("英語データを加工中...")
    with open(en_raw_path, "r", encoding="utf-8") as f:
        text = f.read()
    cleaned = clean(text)
    with open(en_path, "w", encoding="utf-8") as f:
        f.write(f"{cleaned}\n")
    print(f"英語加工データを保存しました: {en_path}")

    # 日本語データの加工
    ja_raw_path = data_path / "ja_raw.txt"
    ja_path = data_path / "ja.txt"

    if not ja_raw_path.exists():
        print(
            f"エラー: {ja_raw_path} が見つかりません。先に 'python dataset.py init' を実行してください。")
        return

    print("日本語データを加工中...")
    with open(ja_raw_path, "r", encoding="utf-8") as f:
        text = f.read().strip()

    kks = pykakasi.kakasi()
    lines = text.split("。")
    ret = ""
    for line in lines:
        if not line.strip():
            continue
        result = kks.convert(line)
        ret += "".join([item["kunrei"] for item in result])
        ret += ". "  # 文の終わりにピリオドを追加

    with open(ja_path, "w", encoding="utf-8") as f:
        f.write(f"{clean(ret)}")
    print(f"日本語加工データを保存しました: {ja_path}")
    print("加工ファイルの作成が完了しました。")


def main():
    """
    メイン関数: コマンドライン引数を解析して適切な処理を実行
    """
    parser = argparse.ArgumentParser(
        description="データセットの取得と加工を行うスクリプト"
    )
    parser.add_argument(
        "command",
        nargs="?",
        choices=["init"],
        help="サブコマンド: init (生データの取得と保存)"
    )
    parser.add_argument(
        "--data-dir",
        default="../data",
        help="データを保存するディレクトリ（デフォルト: ../data）"
    )

    args = parser.parse_args()

    if args.command == "init":
        init_command(args.data_dir)
    else:
        process_command(args.data_dir)


if __name__ == "__main__":
    main()
