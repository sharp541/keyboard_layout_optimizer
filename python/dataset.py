"""
データセットの取得と加工を行うスクリプト

使用方法:
    poetry run python python/dataset.py init  # 生データの取得と保存
    poetry run python python/dataset.py       # 加工ファイルの作成
    poetry run python python/dataset.py analyze-diphthongs
"""
import argparse
from collections import Counter
import json
import os
from pathlib import Path
import re

import pykakasi
from datasets import load_dataset
from huggingface_hub import login


SCRIPT_DIR = Path(__file__).resolve().parent
DEFAULT_DATA_DIR = SCRIPT_DIR.parent / "data"
TOKEN_PATH = SCRIPT_DIR / ".env" / "token.json"
VOWEL_PAIR_ORDER = tuple(first + second for first in "aeiou" for second in "aeiou")
VOWELS = frozenset("aeiou")
NN_PATTERN_ORDER = ("ann", "inn", "unn", "enn", "onn")


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


def extract_vowel_pairs(text):
    """
    テキスト中の連続する母音2文字を数える

    Args:
        text: 集計対象のテキスト

    Returns:
        母音2文字の出現回数
    """
    counts = Counter()
    for first, second in zip(text, text[1:]):
        if first in VOWELS and second in VOWELS:
            counts[first + second] += 1
    return counts


def summarize_vowel_pairs(text):
    """
    テキスト中の母音ペア頻度を集計する

    Args:
        text: 集計対象のテキスト

    Returns:
        (集計結果, 母音ペア総数)
    """
    counts = extract_vowel_pairs(text)
    ordered_counts = Counter({pair: counts.get(pair, 0) for pair in VOWEL_PAIR_ORDER})
    return ordered_counts, sum(ordered_counts.values())


def extract_nn_patterns(text):
    """
    テキスト中の指定3文字パターンを数える

    Args:
        text: 集計対象のテキスト

    Returns:
        指定3文字パターンの出現回数
    """
    counts = Counter()
    for i in range(len(text) - 2):
        token = text[i:i + 3]
        if token in NN_PATTERN_ORDER:
            counts[token] += 1
    return counts


def summarize_nn_patterns(text):
    """
    テキスト中の指定3文字パターン頻度を集計する

    Args:
        text: 集計対象のテキスト

    Returns:
        (集計結果, 指定3文字パターン総数)
    """
    counts = extract_nn_patterns(text)
    ordered_counts = Counter({pattern: counts.get(pattern, 0) for pattern in NN_PATTERN_ORDER})
    return ordered_counts, sum(ordered_counts.values())


def resolve_japanese_analysis_path(data_dir):
    """
    日本語分析対象ファイルのパスを解決する

    Args:
        data_dir: データディレクトリ

    Returns:
        解決済み Path
    """
    data_path = Path(data_dir).resolve()
    return data_path / "ja.txt"


def print_vowel_pair_report(name, counts, total_pairs):
    """
    母音ペア頻度のレポートを表示する

    Args:
        name: レポート名
        counts: 母音ペアの出現回数
        total_pairs: 母音ペア総数
    """
    print(f"[{name}]")
    if total_pairs == 0:
        print("母音2文字の連続は見つかりませんでした。")
        return

    print(f"total_vowel_pairs: {total_pairs}")
    sorted_pairs = sorted(counts.items(), key=lambda item: (-item[1], item[0]))
    for pair, count in sorted_pairs:
        ratio = count / total_pairs
        print(f"{pair}: count={count}, ratio={ratio:.4%}")


def print_nn_pattern_report(name, counts, total_patterns):
    """
    指定3文字パターン頻度のレポートを表示する

    Args:
        name: レポート名
        counts: 指定3文字パターンの出現回数
        total_patterns: 指定3文字パターン総数
    """
    print(f"[{name} nn-patterns]")
    if total_patterns == 0:
        print("指定した3文字パターンは見つかりませんでした。")
        return

    print(f"total_nn_patterns: {total_patterns}")
    sorted_patterns = sorted(counts.items(), key=lambda item: (-item[1], item[0]))
    for pattern, count in sorted_patterns:
        ratio = count / total_patterns
        print(f"{pattern}: count={count}, ratio={ratio:.4%}")


def analyze_diphthongs_command(data_dir=DEFAULT_DATA_DIR):
    """
    加工済み日本語テキストから母音ペアの頻度を分析する

    Args:
        data_dir: データディレクトリ
    """
    path = resolve_japanese_analysis_path(data_dir)

    if not path.exists():
        print(
            f"エラー: {path} が見つかりません。先に 'poetry run python python/dataset.py' を実行してください。"
        )
        return

    with open(path, "r", encoding="utf-8") as f:
        text = f.read()
    pair_counts, total_pairs = summarize_vowel_pairs(text)
    print_vowel_pair_report(path.name, pair_counts, total_pairs)
    print()
    nn_counts, total_patterns = summarize_nn_patterns(text)
    print_nn_pattern_report(path.name, nn_counts, total_patterns)


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
    if not TOKEN_PATH.exists():
        login()
    else:
        with open(TOKEN_PATH, "r", encoding="utf-8") as f:
            config = json.load(f)
            access_key = config["access_key"]
            os.environ["REQUESTS_CA_BUNDLE"] = config["ca_bundle"]
        login(access_key)


def init_command(data_dir=DEFAULT_DATA_DIR):
    """
    initサブコマンド: 生データの取得と保存

    Args:
        data_dir: データを保存するディレクトリ
    """
    print("HuggingFaceにログイン中...")
    login_huggingface()

    data_path = Path(data_dir).resolve()
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


def process_command(data_dir=DEFAULT_DATA_DIR):
    """
    サブコマンド無し: 加工ファイルの作成

    Args:
        data_dir: データが保存されているディレクトリ
    """
    data_path = Path(data_dir).resolve()

    # 英語データの加工
    en_raw_path = data_path / "en_raw.txt"
    en_path = data_path / "en.txt"

    if not en_raw_path.exists():
        print(
            f"エラー: {en_raw_path} が見つかりません。先に 'poetry run python python/dataset.py init' を実行してください。")
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
            f"エラー: {ja_raw_path} が見つかりません。先に 'poetry run python python/dataset.py init' を実行してください。")
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
        choices=["init", "analyze-diphthongs"],
        help="サブコマンド: init (生データの取得と保存), analyze-diphthongs (母音ペア頻度の分析)"
    )
    parser.add_argument(
        "--data-dir",
        default=str(DEFAULT_DATA_DIR),
        help=f"データを保存するディレクトリ（デフォルト: {DEFAULT_DATA_DIR}）"
    )

    args = parser.parse_args()

    if args.command == "init":
        init_command(args.data_dir)
    elif args.command == "analyze-diphthongs":
        analyze_diphthongs_command(args.data_dir)
    else:
        process_command(args.data_dir)


if __name__ == "__main__":
    main()
