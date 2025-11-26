use std::path::Path;
use std::{fs, usize};
use strsim::levenshtein;

#[derive(Debug, Clone)]
enum Token {
    Word(String),
    Separator(char),
}

struct WordList {
    id: String,
    tokens: Vec<Token>,
}

struct SpellChecker {
    dictionary: Vec<String>,
}

impl WordList {
    fn read_and_get(path: &str) -> Result<Vec<WordList>, String> {
        let content = fs::read_to_string(path).map_err(|e| format!("Failed to read: {}", e))?;
        Self::parse_content(&content)
    }

    fn parse_content(content: &str) -> Result<Vec<WordList>, String> {
        content
            .lines()
            .enumerate()
            .map(|(i, line)| (i + 1, line.trim()))
            .filter(|(_, line)| !line.is_empty())
            .map(|(line_num, line)| Self::parse_line(line_num, line))
            .collect::<Result<Vec<WordList>, String>>()
            .and_then(|entries| {
                if entries.is_empty() {
                    Err("cannot find any valid entries".to_string())
                } else {
                    Ok(entries)
                }
            })
    }

    fn parse_line(line_number: usize, line: &str) -> Result<WordList, String> {
        if line.len() < 5 {
            return Err(format!("Line {} is too short: '{}'", line_number, line));
        }

        let id = line[0..4].to_string();

        if !id.chars().all(|c| c.is_ascii_digit()) {
            return Err(format!("Line {} has invalid ID: '{}'", line_number, id));
        }

        let words_part = &line[5..];
        let tokens = Self::parse_tokens(words_part);

        if !tokens.iter().any(|token| matches!(token, Token::Word(_))) {
            return Err(format!(
                "Line {} has no valid words: '{}'",
                line_number, line
            ));
        }

        Ok(WordList { id, tokens })
    }

    fn parse_tokens(words_part: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_word = String::new();

        for char in words_part.chars() {
            if char == ' ' || char == '/' {
                if !current_word.is_empty() {
                    tokens.push(Token::Word(current_word));
                    current_word = String::new();
                }
                tokens.push(Token::Separator(char));
            } else {
                current_word.push(char);
            }
        }

        if !current_word.is_empty() {
            tokens.push(Token::Word(current_word));
        }

        tokens
    }
}

impl SpellChecker {
    fn new(dict_path: &str) -> Result<Self, String> {
        let dict_content = fs::read_to_string(dict_path)
            .map_err(|e| format!("Failed to load dictionary: {}", e))?;

        let mut dictionary: Vec<String> = dict_content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if dictionary.is_empty() {
            return Err("Dictionary is empty".to_string());
        }

        dictionary.sort_unstable();

        Ok(SpellChecker { dictionary })
    }

    fn contains_word(&self, word: &str) -> bool {
        self.dictionary.binary_search(&word.to_string()).is_ok()
    }

    fn correct_word(&self, word: &str) -> String {
        if self.contains_word(word) {
            return word.to_string();
        }

        let mut best_match = word.to_string();
        let mut min_distance = usize::MAX;

        for correct_word in &self.dictionary {
            let distance = levenshtein(word, correct_word);

            if distance < min_distance {
                min_distance = distance;
                best_match = correct_word.clone();

                if distance <= 1 {
                    break;
                }
            }
        }

        best_match
    }

    fn correct_word_list(&self, word_list: &WordList) -> WordList {
        let corrected_tokens: Vec<Token> = word_list
            .tokens
            .iter()
            .map(|token| match token {
                Token::Word(word) => Token::Word(self.correct_word(word)),
                Token::Separator(c) => Token::Separator(*c),
            })
            .collect();

        WordList {
            id: word_list.id.clone(),
            tokens: corrected_tokens,
        }
    }
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Word(word) => write!(f, "{}", word),
            Token::Separator(c) => write!(f, "{}", c),
        }
    }
}

impl std::fmt::Display for WordList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ", self.id)?;
        for token in &self.tokens {
            write!(f, "{}", token)?;
        }
        Ok(())
    }
}

fn main() {
    let word_file = "problem/words.txt";
    let dict_file = "problem/vocabulary.txt";
    let output_file = "problem/correction_words.txt";

    if !Path::new(word_file).exists() {
        println!("Error: File '{}' does not exist", word_file);
        return;
    }

    if !Path::new(dict_file).exists() {
        println!("Error: Dictionary file '{}' does not exist", dict_file);
        return;
    }

    let spell_checker = match SpellChecker::new(dict_file) {
        Ok(checker) => {
            println!(
                "Dictionary loaded successfully with {} words",
                checker.dictionary.len()
            );
            checker
        }
        Err(e) => {
            println!("Failed to initialize spell checker: {}", e);
            return;
        }
    };

    let word_lists = match WordList::read_and_get(word_file) {
        Ok(lists) => {
            println!(
                "Successfully parsed {} entries from {}",
                lists.len(),
                word_file
            );
            lists
        }
        Err(e) => {
            println!("Failed to read word file: {}", e);
            return;
        }
    };

    let corrected_lists: Vec<WordList> = word_lists
        .iter()
        .map(|word_list| spell_checker.correct_word_list(word_list))
        .collect();

    match write_corrected_file(&corrected_lists, output_file) {
        Ok(_) => println!("Correction completed! Result saved to {}", output_file),
        Err(e) => println!("Failed to write output file: {}", e),
    }
}

fn write_corrected_file(word_lists: &[WordList], output_path: &str) -> Result<(), String> {
    if let Some(parent) = Path::new(output_path).parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directory: {}", e))?;
        }
    }

    let content = word_lists
        .iter()
        .map(|word_list| format!("{}\n", word_list))
        .collect::<String>();

    fs::write(output_path, content).map_err(|e| format!("Failed to write file: {}", e))?;

    Ok(())
}
