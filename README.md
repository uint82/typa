# typa

[![GitHub Stars](https://img.shields.io/github/stars/uint82/typa)](https://github.com/uint82/typa)

A minimal, terminal-based typing speed test written in Rust.

Typa is designed to be a lightweight, keyboard-centric alternative to web-based typing tests. It runs entirely in your terminal with support for custom themes and multiple testing modes.

![Recording](./assets/demo.gif)

## Features

- **Multiple Test Modes**:
  - **Time**: Test against a countdown timer (15, 30, 60, 120 seconds).
  - **Words**: Type a set number of words (10, 25, 50, etc.).
  - **Quote**: Type specific quotes from a curated database.

- **Detailed Statistics**: View WPM, raw WPM, accuracy, and character breakdowns (correct/incorrect/extra/missed) after every test run.

- **Customization**:
  - Full color theme support via TOML configuration file.
  - Toggle punctuation and numbers independently.
  - Multiple language support (English and Indonesian).

- **Responsive UI**:
  - Clean, distraction-free interface.
  - Line wrapping that adapts to terminal width.

## Installation

### Install with Cargo

```bash
cargo install typa
```

### Build from Source

1. **Clone Repository**

```bash
git clone https://github.com/uint82/typa.git
cd typa
```

2. **Build the project**

```bash
cargo install --path .
```

3. **Run the binary**

```bash
./target/release/typa
```

## Usage

To start the default test (Time mode, 60 seconds, English):

```bash
typa
```

For usage instructions, run:

```bash
typa --help
```

### Command-Line Options

```
typa 0.1.0

A rusty terminal typing test

Usage: typa [OPTIONS]

Options:
  -t, --time <TIME>          Time mode: Custom duration in seconds (e.g. 15, 60, 120, 3600)
  -w, --words <WORDS>        Words mode: Word count (1 to 10000)
  -q, --quote <QUOTE>        Quote mode: "short", "medium", "long", "very_long", "all", or a specific ID (e.g. 25)
  -l, --language <LANGUAGE>  Language: Filename to use (e.g. "english", "indonesian") [default: english]

Flags:
  -n, --numbers      Include numbers in the test
  -p, --punctuation  Include punctuation in the test
  -h, --help         Print help
  -V, --version      Print version
```

### Examples

```bash
# Run a 60 second test
typa -t 60

# Run a 50 word test
typa -w 50

# Run a short quote test
typa -q short

# Run a 30 second test with punctuation and numbers
typa -t 30 -p -n

# Run a 100 word test in Indonesian with punctuation
typa -w 100 -l indonesian -p

# Run a specific quote by ID
typa -q 42

# Run a very long quote test
typa -q verylong
```

## Keyboard Shortcuts

During a test:

- **Tab**: Restart the current test
- **Esc** or **Ctrl+Q**: Quit the application

## Configuration

Typa supports custom color themes via a TOML configuration file.

### Configuration File Location

The configuration file should be named `config.toml` and placed in:

- **Linux**: `~/.config/typa/config.toml`
- **macOS**: `$HOME/Library/Application Support/typa/config/config.toml`
- **Windows**: `C:\Users\user\AppData\Roaming\typa\config\config.toml`

**Note**: If the configuration directory doesn't exist, you'll need to create it manually before adding your `config.toml` file.

### Example Configuration

```toml
[theme]
bg = "#2c2e34"          # Background color
main = "#e2b714"        # Brand color (timer, active stats, highlights)
caret = "#e2b714"       # Cursor block color
text = "#d1d0c5"        # Correctly typed text
sub = "#646669"         # Untyped text, UI labels, footer instructions
sub_alt = "#45474d"     # UI borders, subtle elements
error = "#ca4754"       # Incorrect / extra characters
```

All colors should be specified in hexadecimal format. If the configuration file is not found, default colors will be used.

## Statistics Explanation

After completing a test, you'll see several metrics:

- **WPM (Words Per Minute)**: Your typing speed adjusted for accuracy. Calculated as `(correct_chars / 5 - uncorrected_errors) / time_in_minutes`.
- **Raw WPM**: Your typing speed without accuracy adjustments. Calculated as `(all_typed_chars / 5) / time_in_minutes`.
- **Accuracy**: Percentage of characters typed correctly: `(correct_chars / total_chars) × 100`.
- **Character Breakdown**:
  - **cor**: Correctly typed characters
  - **inc**: Incorrectly typed characters
  - **ext**: Extra characters typed beyond the expected text
  - **mis**: Characters you skipped or didn't type
- **Time**: Total time spent on the test in seconds

## Quote Mode Details

Quote mode allows you to type passages from a curated collection. Quotes are categorized by length:

- **Short**: 0 - 100 words
- **Medium**: 101 - 300 words
- **Long**: 301 - 600 words
- **Very Long**: 601 - 9999 words
- **All**: Random selection from all categories

You can also select a specific quote by its ID number if you know it.

## Language Support

Typa includes word lists and quote collections for multiple languages. The default is English, but you can specify others using the `-l` flag.

Currently supported languages:

- English (`english`)
- Indonesian (`indonesian`)

Language files are embedded in the binary and include both word lists for generating tests and curated quotes for quote mode.

## Contributing

Contributions are welcome! Here's how you can help:

### How to Contribute

1. **Fork the repository** on GitHub
2. **Create a feature branch** (`git checkout -b feat/amazing-feature`)
3. **Make your changes** and commit them using [Conventional Commits](https://www.conventionalcommits.org/)
   - `feat:` for new features
   - `fix:` for bug fixes
   - `docs:` for documentation changes
   - `style:` for formatting, missing semicolons, etc.
   - `refactor:` for code refactoring
   - `test:` for adding tests
   - `chore:` for maintenance tasks
4. **Push to your branch** (`git push origin feat/amazing-feature`)
5. **Open a Pull Request**

### Commit Message Examples

```bash
feat: add support for Spanish language
fix: correct WPM calculation for long tests
docs: update installation instructions
chore: update dependencies
```

### Areas for Improvement

- Adding more language support
- Expanding quote collections
- Additional theme presets
- Performance optimizations
- Bug fixes and testing
- Adding test history

### Reporting Issues

Found a bug or have a feature request? Please [open an issue](https://github.com/uint82/typa/issues) on GitHub with:

- A clear description of the problem or suggestion
- Steps to reproduce (for bugs)
- Your environment details (OS, terminal, Rust version)

All contributions, big or small, are appreciated!

## License

This project is licensed under the GNU General Public License v3.0. See the [LICENSE](LICENSE) file for details.

## Acknowledgments

Inspired by popular typing test platforms like Monkeytype and tt, but designed for terminal enthusiasts who prefer a minimal, keyboard-driven experience.

This project uses open-source word lists and quote data sourced from the [Monkeytype GitHub repository](https://github.com/monkeytype/monkeytype), which is licensed under GPL-3.0. All rights to the original content belong to their respective authors. This project is not affiliated with or endorsed by Monkeytype.
