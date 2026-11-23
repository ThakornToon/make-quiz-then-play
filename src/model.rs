use serde::{Deserialize, Serialize};

use crate::AppResult;

pub const MAX_CHOICES: usize = 26;

// ข้อมูลหลักของ quiz โดย blocks เก็บโจทย์และกลุ่มตามลำดับที่ผู้สร้างเพิ่มไว้
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Quiz {
    // ชื่อที่ใช้แสดงในเมนูและในรายงานผล
    pub title: String,
    // รายการเนื้อหา; แต่ละสมาชิกเป็นโจทย์เดี่ยวหรือกลุ่มโจทย์
    pub blocks: Vec<QuizBlock>,
}

/// ตำแหน่งของโจทย์ใน quiz; None หมายถึงบล็อกโจทย์เดี่ยว
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuestionLocation {
    pub block_index: usize,
    pub question_index: Option<usize>,
}

impl Quiz {
    /// สร้างตำแหน่งของทุกโจทย์ตามลำดับที่ปรากฏใน quiz
    pub fn question_locations(&self) -> Vec<QuestionLocation> {
        let mut locations = Vec::new();
        for (block_index, block) in self.blocks.iter().enumerate() {
            match block {
                QuizBlock::Question { .. } => locations.push(QuestionLocation {
                    block_index,
                    question_index: None,
                }),
                QuizBlock::Group { questions, .. } => {
                    for question_index in 0..questions.len() {
                        locations.push(QuestionLocation {
                            block_index,
                            question_index: Some(question_index),
                        });
                    }
                }
            }
        }
        locations
    }

    /// คืนโจทย์ตามตำแหน่ง หากตำแหน่งไม่ตรงกับโครงสร้าง quiz จะคืน None
    pub fn question_at(&self, location: QuestionLocation) -> Option<&Question> {
        match (
            self.blocks.get(location.block_index)?,
            location.question_index,
        ) {
            (QuizBlock::Question { question }, None) => Some(question),
            (QuizBlock::Group { questions, .. }, Some(question_index)) => {
                questions.get(question_index)
            }
            _ => None,
        }
    }

    /// คืน mutable reference ของโจทย์ตามตำแหน่ง หากตำแหน่งไม่ถูกต้องจะคืน None
    pub fn question_at_mut(&mut self, location: QuestionLocation) -> Option<&mut Question> {
        match (
            self.blocks.get_mut(location.block_index)?,
            location.question_index,
        ) {
            (QuizBlock::Question { question }, None) => Some(question),
            (QuizBlock::Group { questions, .. }, Some(question_index)) => {
                questions.get_mut(question_index)
            }
            _ => None,
        }
    }
}

// หน่วยของเนื้อหาที่สามารถสลับลำดับได้ โดยกลุ่มจะถูกสลับเป็นก้อนเดียว
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]  // ใช้ tag "type" เพื่อระบุชนิดของ block ในไฟล์ JSON และแปลงชื่อ variant เป็นตัวพิมพ์เล็กคั่นด้วย underscore
pub enum QuizBlock {
    Question {
        // โจทย์เดี่ยวที่ไม่อยู่ในกลุ่ม
        question: Question,
    },
    Group {
        // ชื่อกลุ่มที่แสดงกำกับบทความและโจทย์ย่อย
        title: String,
        // เนื้อหาประกอบกลุ่ม ซึ่งอาจมีหลายบรรทัด
        passage: String,
        // โจทย์ย่อยภายในกลุ่ม; ลำดับใน vector จะไม่ถูกสลับ
        questions: Vec<Question>,
    },
}

// ข้อมูลโจทย์หนึ่งข้อ พร้อม choice และข้อมูลเฉลย
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Question {
    // ข้อความโจทย์ ซึ่งอาจประกอบด้วยหลายบรรทัด
    pub prompt: String,
    // ข้อความตัวเลือกตามลำดับเดิมที่บันทึกในไฟล์
    pub choices: Vec<String>,
    // index ของตัวเลือกคำตอบที่ถูกต้อง โดยเริ่มนับจากศูนย์
    pub answer: Option<usize>,
    // คำอธิบายเฉลยที่ไม่บังคับกรอก
    pub explanation: Option<String>,
    // ระบุว่าควรสุ่มลำดับตัวเลือกเมื่อเริ่มทำ quiz หรือไม่
    pub shuffle_choices: bool,
}

// นำเฉลยและคำอธิบายออกจาก quiz ก่อนส่งไฟล์ให้นักเรียน
pub fn remove_answer_key(quiz: &mut Quiz) {
    for block in &mut quiz.blocks {
        let questions = match block {
            QuizBlock::Question { question } => std::slice::from_mut(question),
            QuizBlock::Group { questions, .. } => questions,
        };
        for question in questions {
            question.answer = None;
            question.explanation = None;
        }
    }
}

// ไฟล์คำตอบที่ส่งแยกจาก quiz เพื่อให้ตรวจด้วยไฟล์เฉลยบนอีกเครื่องได้
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AnswerSubmission {
    pub format_version: u32,
    pub quiz_title: String,
    pub student_name: Option<String>,
    pub answers: Vec<SubmittedAnswer>,
}

// คำตอบหนึ่งข้อ พร้อมข้อมูลระบุตำแหน่งและโครงสร้างสำหรับตรวจว่าใช้ quiz เฉลยชุดเดียวกัน
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmittedAnswer {
    pub block_index: usize,
    pub question_index: Option<usize>,
    pub group_title: Option<String>,
    pub passage: Option<String>,
    pub prompt: String,
    pub choices: Vec<String>,
    pub selected: Option<usize>,
}

// ตรวจความถูกต้องของ quiz ที่อ่านจากไฟล์ก่อนนำไปใช้งาน
pub fn validate_quiz(quiz: &Quiz) -> AppResult<()> {
    if quiz.title.trim().is_empty() || quiz.blocks.is_empty() {
        // .into() แปลงข้อความ error ให้เป็นชนิด Box<dyn Error>
        return Err("ไฟล์ quiz ต้องมีชื่อและโจทย์อย่างน้อยหนึ่งข้อ".into());
    }
    // ตรวจทั้งโจทย์เดี่ยวและโจทย์ย่อยในทุกกลุ่ม
    for block in &quiz.blocks {
        match block {
            QuizBlock::Question { question } => validate_question(question)?,
            QuizBlock::Group {
                title, questions, ..
            } => {
                if title.trim().is_empty() || questions.is_empty() {
                    return Err("กลุ่มต้องมีชื่อและโจทย์อย่างน้อยหนึ่งข้อ".into());
                }
                for question in questions {
                    validate_question(question)?;
                }
            }
        }
    }
    Ok(())
}

// ตรวจว่าข้อความ choice และ index เฉลยอยู่ในช่วงที่ใช้งานได้
pub fn validate_question(question: &Question) -> AppResult<()> {
    if question.prompt.trim().is_empty()
        || question.choices.len() < 2
        || question.choices.len() > MAX_CHOICES
        || question
            .choices
            .iter()
            .any(|choice| choice.trim().is_empty())
        || question
            .answer
            .is_some_and(|answer| answer >= question.choices.len())
    {
        // .into() แปลงข้อความ error ให้เป็นชนิด Box<dyn Error>
        return Err("พบข้อมูลโจทย์หรือ choice ที่ไม่ถูกต้องในไฟล์ quiz".into());
    }
    Ok(())
}

