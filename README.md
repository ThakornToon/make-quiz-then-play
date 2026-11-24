# Quiz CLI 🦀

A powerful, interactive command-line quiz generator, runner, and grading suite written in Rust.

Designed for educators and learners, **Quiz CLI** supports reading passages with grouped subquestions, flexible choice and question shuffling, safe interactive quiz authoring, and a secure **Teacher-Student workflow** that decouples question papers from answer keys.

---

## ✨ Features

- **Decoupled Teacher / Student Workflow**:
  - Export stripped student copies (`exports/`) that contain no answer keys or explanations.
  - Students run quizzes locally and generate verifiable answer submission files (`submissions/`).
  - Teachers grade submissions using the master key file, producing comprehensive score summaries and text reports (`results/`).
  - Built-in validation prevents grading submissions against mismatched quiz versions or questions.
- **Rich Question Types & Grouped Passages**:
  - **Single Questions (`q`)**: Standalone multiple-choice questions (up to 26 choices).
  - **Grouped Questions (`g`)**: Reading comprehension or scenario-based passages with bundled subquestions that stay together.
  - Multi-line support for prompts and passages (end input with a single `.` on its own line).
- **Smart Randomization**:
  - **Block Shuffling**: Randomize question presentation order while keeping grouped questions and passages together.
  - **Per-Question Choice Shuffling**: Toggle choice randomization per question (ideal for keeping "None of the above" or sequential choices in order).
- **Safe In-Memory Editor**:
  - Rename quizzes, edit prompts, update choices, change answer keys, and write explanations.
  - Reorder questions, move questions in/out/between groups, or reorder entire groups.
  - Add or delete questions with confirmation.
  - Changes remain in memory until explicitly confirmed, protecting existing JSON files.
- **Interactive Quiz Runner**:
  - Intuitive navigation: step forward/backward, jump directly to any question, change or clear answers.
  - Unanswered question alerts before final submission.
  - **Cancel mid-quiz** (`q` / `quit` / `cancel`) to abort without saving — with confirmation prompt.
  - Instant on-screen grading summary and exported report.
- **Cancel / Go Back at Every Step**:
  - Every selection menu (quiz, question, group, file) supports `0` to cancel and return.
  - Create-quiz flow supports `cancel` to discard unsaved work.
  - Move operations safely restore data to its original position when cancelled.

---

## Getting Started

### Prerequisites

- [Rust & Cargo](https://www.rust-lang.org/tools/install)

### Installation & Run

Clone the repository and run via Cargo:

```sh
# Clone repository
git clone <repo>

# Run in development mode
cargo run

# Or build an optimized release binary
cargo build --release
./target/release/quiz-cli
```

---

## 🔄 Teacher vs. Student Workflow

```
[ Teacher ]                              [ Student ]
     │                                        │
 1. Create Quiz with Answer Key               │
     │                                        │
 2. Export Student Copy (No Key)              │
    └─► File saved to exports/ ──[Distribute]─┼─► Placed in quizzes/
                                              │
                                         3. Take Quiz
                                              │
                                         4. Submit Answers
    ┌─[Send Back]─────────────────────────────┴─► File saved to submissions/
    ▼
 5. Grade Submission (Master Key)
    └─► Report saved to results/
```

1. **Teacher Creates Master Quiz**: Create quiz with questions, correct answers, and explanations. Saved to `quizzes/`.
2. **Teacher Exports Student Copy**: Run `Export quiz for students`. An answer-key-stripped copy is generated in `exports/`.
3. **Student Runs Quiz**: Place the exported JSON file in the student's `quizzes/` folder. The student takes the quiz.
4. **Student Submits**: Upon completion, a submission file is created in `submissions/` (contains selected choices and validation metadata, but no answer keys).
5. **Teacher Grades**: Transfer the submission file to the teacher's machine under `submissions/`. Select `Grade answer file` with the original master quiz. A detailed grading report is generated in `results/`.

---

## 🎮 During a Quiz (Controls)

When taking a quiz, use the following terminal commands:

| Command | Action |
|---|---|
| `<number>` | Select choice number to answer the current question |
| `c` / `clear` / `ลบ` | Clear / deselect answer for the current question |
| `j <number>` | Jump directly to question `<number>` (e.g., `j 5`) |
| `<` | Move to previous question |
| `>` | Move to next question |
| `s` | Submit quiz (warns if any question is unanswered) |
| `q` / `quit` / `cancel` / `ยกเลิก` | Abort quiz and return to main menu (with confirmation; answers are **not** saved) |
| `?` / `help` | Show in-game command assistance |

---

## 📁 Project & Storage Structure

```
quizzz/
├── Cargo.toml            # Project manifest & dependencies (rand, serde, serde_json)
├── src/
│   ├── main.rs           # Program entry point and top-level CLI menu
│   ├── model.rs          # Data structures (Quiz, Block, Question, Submission) & validation
│   ├── input.rs          # Terminal I/O utilities, multi-line prompts & input validation
│   ├── editor.rs         # Interactive quiz creator, editor, and student exporter
│   ├── play.rs           # Quiz runner, navigation loop, submission exporter & grader
│   └── storage.rs        # Safe atomic JSON persistence, directory & path management
│
├── quizzes/              # Master and active quiz JSON files
├── exports/              # Student copies (stripped of answer keys & explanations)
├── submissions/          # Answer submission files (.json)
└── results/              # Detailed text grading reports (.txt)
```

---

## 🛠️ Main Menu Options

1. **1) เพิ่ม quiz (Create Quiz)**: Create single questions or grouped questions with reading passages. Supports multi-line input (terminated with `.`). Use `cancel` or `x` to abort creation without saving.
2. **2) ทำ quiz (Take Quiz)**: Select a quiz, optionally shuffle questions, and take the quiz interactively. Use `q` to abort mid-quiz.
3. **3) แก้ไข quiz (Edit Quiz)**: Edit title, prompts, choices, explanations, shuffle settings, reorder questions/blocks, or delete questions. All sub-menus support `0` to cancel.
4. **4) ตรวจไฟล์คำตอบ (Grade Submission)**: Cross-verify a student submission file against the master quiz and produce an itemized score report.
5. **5) ส่งออก quiz สำหรับนักเรียน (Export Student Copy)**: Generate a clean, answer-free JSON copy into `exports/`.
6. **6) ลบ quiz (Delete Quiz)**: Remove an existing quiz with confirmation.
7. **0) ออกจากโปรแกรม (Exit)**.

> **Tip:** At any quiz or file selection prompt, enter `0` to cancel and return to the previous menu.

---

## License

This project is open-source. Feel free to use, modify, and distribute it.
