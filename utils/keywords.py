#!/usr/bin/env python3
"""
Keywords Generator for Open-XiaoAI KWS
Converts Chinese text to pinyin format for Sherpa-ONNX wake word recognition
"""

import argparse
import sys
from typing import List, Tuple

try:
    from pypinyin import pinyin, Style
except ImportError:
    print("❌ Error: pypinyin is required")
    print("📥 Install with: pip install pypinyin")
    sys.exit(1)


def text_to_pinyin_keywords(text: str) -> str:
    """Convert Chinese text to pinyin format for KWS"""
    # Get pinyin with tone marks
    pinyin_list = pinyin(text, style=Style.TONE3, neutral_tone_with_five=True)
    
    # Join pinyin syllables with spaces
    pinyin_str = " ".join([item[0] for item in pinyin_list])
    
    return f"{pinyin_str} @{text}"


def process_keywords_file(input_file: str, output_file: str, tokens_file: str = None) -> None:
    """Process a file of keywords and generate KWS format"""
    try:
        with open(input_file, 'r', encoding='utf-8') as f:
            lines = f.readlines()
    except FileNotFoundError:
        print(f"❌ Error: Input file not found: {input_file}")
        sys.exit(1)
    
    keywords = []
    
    for line in lines:
        line = line.strip()
        if not line or line.startswith('#'):
            continue
        
        # Convert to KWS format
        kws_line = text_to_pinyin_keywords(line)
        keywords.append(kws_line)
        print(f"✅ Converted: {line} -> {kws_line}")
    
    # Write output file
    try:
        with open(output_file, 'w', encoding='utf-8') as f:
            for keyword in keywords:
                f.write(keyword + '\n')
        print(f"📝 Output written to: {output_file}")
    except Exception as e:
        print(f"❌ Error writing output file: {e}")
        sys.exit(1)
    
    # Write tokens file if requested (compatibility with original script)
    if tokens_file:
        try:
            with open(tokens_file, 'w', encoding='utf-8') as f:
                for keyword in keywords:
                    # Extract pinyin part (before @)
                    pinyin_part = keyword.split('@')[0].strip()
                    f.write(pinyin_part + '\n')
            print(f"📝 Tokens written to: {tokens_file}")
        except Exception as e:
            print(f"❌ Error writing tokens file: {e}")


def main():
    parser = argparse.ArgumentParser(
        description="Generate KWS keywords from Chinese text",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python keywords.py --text my-keywords.txt --output keywords.txt
  python keywords.py --text my-keywords.txt --output keywords.txt --tokens tokens.txt
  echo "小爱老师" | python keywords.py --stdin --output keywords.txt
        """
    )
    
    parser.add_argument('--text', '-t', help='Input file with Chinese keywords (one per line)')
    parser.add_argument('--output', '-o', required=True, help='Output file for KWS format')
    parser.add_argument('--tokens', help='Optional tokens output file (for compatibility)')
    parser.add_argument('--stdin', action='store_true', help='Read from stdin instead of file')
    
    args = parser.parse_args()
    
    if args.stdin:
        print("📥 Reading from stdin (Ctrl+D to finish):")
        lines = sys.stdin.readlines()
        keywords = []
        
        for line in lines:
            line = line.strip()
            if not line or line.startswith('#'):
                continue
            
            kws_line = text_to_pinyin_keywords(line)
            keywords.append(kws_line)
            print(f"✅ Converted: {line} -> {kws_line}")
        
        # Write output
        try:
            with open(args.output, 'w', encoding='utf-8') as f:
                for keyword in keywords:
                    f.write(keyword + '\n')
            print(f"📝 Output written to: {args.output}")
        except Exception as e:
            print(f"❌ Error writing output: {e}")
            sys.exit(1)
    
    elif args.text:
        process_keywords_file(args.text, args.output, args.tokens)
    
    else:
        print("❌ Error: Must specify either --text or --stdin")
        parser.print_help()
        sys.exit(1)


if __name__ == "__main__":
    main()
