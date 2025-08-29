# Contributing to WooDns

Thank you for considering contributing to **WooDns**! 🎉
Contributions help make this project better, and we welcome all types of help — whether it’s fixing bugs, adding new features, improving documentation, or sharing ideas.

---

## 📋 Table of Contents

* [How Can I Contribute?](#-how-can-i-contribute)

  * [Reporting Bugs](#reporting-bugs)
  * [Suggesting Features](#suggesting-features)
  * [Improving Documentation](#improving-documentation)
  * [Submitting Code](#submitting-code)
* [Getting Started](#-getting-started)
* [Development Workflow](#-development-workflow)
* [Code Style](#-code-style)
* [Commit Guidelines](#-commit-guidelines)
* [Pull Request Process](#-pull-request-process)
* [Community](#-community)
* [Rust Version Requirement](#-rust-version-requirement)

---

## 🙌 How Can I Contribute?

### Reporting Bugs

* Search the [issues](../../issues) to make sure it hasn’t already been reported.
* If not found, [open a new issue](../../issues/new) and include:

  * A clear title and description.
  * Steps to reproduce the issue.
  * Expected vs actual behavior.
  * Environment details (OS, version, etc.).

### Suggesting Features

* Check if there is an existing feature request.
* Clearly describe:

  * The problem your feature solves.
  * How it would work.
  * Possible alternatives.

### Improving Documentation

* Fix typos, grammar, or clarity.
* Add examples, tutorials, or detailed explanations.

### Submitting Code

* Pick an open issue or create one for discussion.
* Make sure the change aligns with the project’s goals.

---

## 🚀 Getting Started

1. **Fork** the repository.
2. **Clone** your fork:

   ```bash
   git clone https://github.com/<your-username>/WooDns.git
   cd WooDns
   ```
3. **Create a new branch** for your work:

   ```bash
   git checkout -b feature/my-feature
   ```
4. **Install dependencies** (if any).
5. Make your changes.

---

## 🛠 Development Workflow

* Keep commits small and focused.
* Regularly **sync with `main`** to avoid conflicts.
* Write tests when possible.
* Run all tests before pushing changes.

---

## 🎨 Code Style

* Follow [Rust guidelines](https://doc.rust-lang.org/1.0.0/style/).
* Use `cargo fmt` to format code.
* Run `cargo clippy` to check for common mistakes.

---

## ✍️ Commit Guidelines

We use conventional commits for clarity:

* `feat:` – A new feature.
* `fix:` – A bug fix.
* `docs:` – Documentation updates.
* `style:` – Code style changes (formatting, etc.).
* `refactor:` – Code refactoring.
* `test:` – Adding/updating tests.
* `chore:` – Maintenance tasks.

Example:

```bash
git commit -m "feat: add support for AAAA DNS record"
```

---

## 🔀 Pull Request Process

1. Ensure your code follows style and passes tests.
2. Update documentation if needed.
3. Link related issues in your PR description.
4. Request a review from maintainers.
5. Be open to feedback and iterate.

---

## 🌍 Community

* Be respectful and constructive.
* Encourage newcomers.
* Collaborate with kindness.

---

## 🦀 Rust Version Requirement

WooDns requires **Rust 1.89.0 or later**.
Please ensure your Rust toolchain is up to date:

```bash
rustup update
```

You can check your version with:

```bash
rustc --version
```

---

💡 **Tip:** Start with issues labeled `good first issue` or `help wanted`.

Thank you for contributing to **WooDns**! 🚀
