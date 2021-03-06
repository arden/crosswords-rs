use cw::{CVec, BLOCK};
use rand;
use rand::Rng;
use std::ascii::AsciiExt;
use std::cmp;
use std::collections::{HashMap, HashSet};
use std::slice;
use word_constraint::WordConstraint;

fn matches(word: &CVec, pattern: &CVec) -> bool {
    word.len() <= pattern.len()
        && word.iter().zip(pattern.iter()).all(|(&cw, &cp)| cw == cp || cp == BLOCK)
}

/// An iterator over all words satisfying a given `WordConstraint`.
pub struct PatternIter<'a> {
    dict: &'a Dict,
    pattern: CVec,
    list: &'a Vec<usize>,
    index: usize,
}

impl<'a> PatternIter<'a> {
    fn get_word(&self) -> Option<&'a CVec> {
        self.list.get(self.index).and_then(|&i| self.dict.words.get(i))
    }
}

impl<'a> Iterator for PatternIter<'a> {
    type Item = &'a CVec;

    fn next(&mut self) -> Option<&'a CVec> {
        loop {
            let word = self.get_word();
            self.index += 1;
            if word.iter().all(|w| matches(w, &self.pattern)) {
                return word;
            }
        }
    }
}

/// A `Dict` stores a list of words - represented as char vectors - and indexes them for
/// efficiently iterating over all words satisfying a given `WordConstraint`.
pub struct Dict {
    words: Vec<CVec>,
    lists: HashMap<WordConstraint, Vec<usize>>,
    max_n: usize,
    empty_list: Vec<usize>,
}

impl Dict {
    /// Return a `HashSet` of the given words, as char vectors, replacing umlauts with
    /// corresponding diphthongs and deduplicating the words.
    pub fn to_cvec_set<T: Iterator<Item = String>>(string_words: T) -> HashSet<CVec> {
        string_words.filter_map(Dict::normalize_word).collect()
    }

    /// Create a new `Dict` from the given sequence of words.
    pub fn new<'a, T: Iterator<Item = &'a CVec>>(all_words: T) -> Dict {
        let mut dict = Dict {
            words: all_words.cloned().collect(),
            lists: HashMap::new(),
            max_n: 3, // TODO: Make this a parameter?
            empty_list: Vec::new(),
        };
        let mut rng = rand::thread_rng(); // TODO: Make this a parameter
        rng.shuffle(&mut dict.words[..]);
        for (i, word) in dict.words.iter().enumerate() {
            for woco in WordConstraint::all(word, dict.max_n){
                if !dict.lists.get(&woco).is_some() {
                    dict.lists.insert(woco.clone(), vec!(i));
                } else {
                    dict.lists.get_mut(&woco).unwrap().push(i);
                }
            }
        }
        dict
    }

    fn normalize_word(string_word: String) -> Option<CVec> {
        // TODO: Use to_uppercase() once it's stable.
        let word: CVec = string_word.to_ascii_uppercase().trim()
                       .replace("ä", "AE")
                       .replace("Ä", "AE")
                       .replace("ö", "OE")
                       .replace("Ö", "OE")
                       .replace("ü", "UE")
                       .replace("Ü", "UE")
                       .replace("ß", "SS").chars().collect();
        if word.iter().all(|&c| c.is_alphabetic() && c.is_ascii()) && word.len() > 1 {
            Some(word)
        } else {
            None
        }
    }

    fn get_list<'a>(&'a self, wc: &WordConstraint) -> &'a Vec<usize> {
        self.lists.get(wc).unwrap_or(&self.empty_list)
    }

    /// Return whether the given word is present in this dictionary.
    pub fn contains(&self, word: &CVec) -> bool {
        self.matching_words(word.clone()).next().is_some()
    }

    /// Return an iterator over all words in the dictionary.
    pub fn all_words<'a>(&'a self) -> slice::Iter<'a, CVec> {
        self.words.iter()
    }

    fn get_matching_word_list<'a>(&'a self, pattern: &CVec) -> &'a Vec<usize> {
        let len = pattern.len();
        let mut list: &'a Vec<usize> = self.get_list(&WordConstraint::Length(pattern.len()));
        if list.len() == 0 {
            return list;
        }
        let mut pos = 0;
        for i in pattern.iter().enumerate()
                .filter(|&(_, ch)| ch == &BLOCK)
                .map(|(i, _)| i)
                .chain(Some(len).into_iter()) {
            if i > pos {
                let subword = &pattern[pos..i];
                let n = cmp::min(self.max_n, subword.len());
                for dp in 1..(subword.len() - n) {
                    let wc = WordConstraint::with_ngram(&subword[dp..(dp + n)], pos + dp, len);
                    let new_list = self.get_list(&wc);
                    if list.len() > new_list.len() {
                        list = new_list;
                        if list.len() == 0 {
                            return list;
                        }
                    }
                }
            }
            pos = i + 1;
        }
        list
    }

    /// Return an iterator over all words in the dictionary matching the given pattern.
    pub fn matching_words<'a>(&'a self, pattern: CVec) -> PatternIter<'a> {
        let list = self.get_matching_word_list(&pattern);
        PatternIter {
            dict: self,
            pattern: pattern,
            list: list,
            index: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let words_vec = vec!("FOO", "FOOBAR", "FOE", "TOE");
        let words = Dict::to_cvec_set(words_vec.into_iter().map(|s| s.to_string()));
        let dict = Dict::new(words.iter());
        assert_eq!(2, dict.matching_words("#OE".chars().collect()).count());
        assert_eq!(1, dict.matching_words("F#E".chars().collect()).count());
        assert_eq!(0, dict.matching_words("T#O".chars().collect()).count());
        assert_eq!(0, dict.matching_words("F###".chars().collect()).count());
        assert_eq!(0, dict.matching_words("##".chars().collect()).count());
    }
}

