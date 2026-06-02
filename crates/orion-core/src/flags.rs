use crate::config::{wrap_index, Direction};
use crate::generated::flags_assets::{FLAG_ASSETS, FLAG_ASSET_COUNT};
use crate::rng::Rng;
use crate::store::HighScoreStore;

pub const FLAGS_OPTION_COUNT: usize = 4;
pub const FLAGS_QUIZ_ROUNDS: u32 = 20;
pub const FLAGS_FEEDBACK_MS: u64 = 900;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagAsset {
    pub code: &'static str,
    pub name: &'static str,
    pub width: u16,
    pub height: u16,
    pub offset: u32,
}

pub const SAMPLE_FLAGS: &[FlagAsset] = &[
    FlagAsset {
        code: "AD",
        name: "ANDORRA",
        width: 148,
        height: 104,
        offset: 0,
    },
    FlagAsset {
        code: "AE",
        name: "UNITED ARAB EMIRATES",
        width: 160,
        height: 80,
        offset: 30_784,
    },
    FlagAsset {
        code: "AF",
        name: "AFGHANISTAN",
        width: 155,
        height: 104,
        offset: 56_384,
    },
    FlagAsset {
        code: "AG",
        name: "ANTIGUA AND BARBUDA",
        width: 155,
        height: 104,
        offset: 88_624,
    },
    FlagAsset {
        code: "AI",
        name: "ANGUILLA",
        width: 160,
        height: 80,
        offset: 120_864,
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagsMode {
    Practice,
    Quiz20,
    DeathMatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagsState {
    ChoosingMode,
    Question,
    Feedback,
    Paused,
    Results,
    Over,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagsResultAction {
    Restart,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagsPauseAction {
    Continue,
    Exit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagsChoosingAction {
    Mode(FlagsMode),
    Exit,
}

#[derive(Debug, Clone)]
pub struct FlagsGame {
    flag_count: usize,
    state: FlagsState,
    selected_mode: FlagsMode,
    active_mode: FlagsMode,
    result_action: FlagsResultAction,
    pause_action: FlagsPauseAction,
    choosing_action: FlagsChoosingAction,
    answer_indices: [usize; FLAGS_OPTION_COUNT],
    used_correct_flags: [bool; FLAG_ASSET_COUNT],
    correct_flag_index: usize,
    selected_answer: usize,
    correct_answer: usize,
    last_answer_correct: bool,
    score: u32,
    round: u32,
    best_score: u32,
    used_correct_flag_count: usize,
}

impl FlagsGame {
    pub fn new(flag_count: usize) -> Self {
        assert!(flag_count >= FLAGS_OPTION_COUNT);
        assert!(flag_count <= FLAG_ASSET_COUNT);
        Self {
            flag_count,
            state: FlagsState::ChoosingMode,
            selected_mode: FlagsMode::Practice,
            active_mode: FlagsMode::Practice,
            result_action: FlagsResultAction::Restart,
            pause_action: FlagsPauseAction::Continue,
            choosing_action: FlagsChoosingAction::Mode(FlagsMode::Practice),
            answer_indices: [0; FLAGS_OPTION_COUNT],
            used_correct_flags: [false; FLAG_ASSET_COUNT],
            correct_flag_index: 0,
            selected_answer: 0,
            correct_answer: 0,
            last_answer_correct: false,
            score: 0,
            round: 0,
            best_score: 0,
            used_correct_flag_count: 0,
        }
    }

    pub fn enter_choosing(&mut self, high_scores: &impl HighScoreStore) {
        self.state = FlagsState::ChoosingMode;
        self.result_action = FlagsResultAction::Restart;
        self.pause_action = FlagsPauseAction::Continue;
        self.choosing_action = FlagsChoosingAction::Mode(self.selected_mode);
        self.score = 0;
        self.round = 0;
        self.refresh_best_score(high_scores);
    }

    pub fn start_selected_mode(&mut self, high_scores: &impl HighScoreStore, rng: &mut impl Rng) {
        self.active_mode = self.selected_mode;
        self.result_action = FlagsResultAction::Restart;
        self.score = 0;
        self.round = 0;
        self.reset_question_history();
        self.refresh_best_score(high_scores);
        self.next_question(rng);
    }

    pub fn cycle_mode(&mut self, detents: i32) -> bool {
        if self.state != FlagsState::ChoosingMode || detents == 0 {
            return false;
        }
        let next = cycle_mode_from(self.selected_mode, detents);
        if next == self.selected_mode {
            false
        } else {
            self.selected_mode = next;
            true
        }
    }

    pub fn select_previous_mode(&mut self) -> bool {
        self.cycle_mode(-1)
    }

    pub fn select_next_mode(&mut self) -> bool {
        self.cycle_mode(1)
    }

    pub fn move_answer_selection(&mut self, direction: Direction) -> bool {
        if self.state != FlagsState::Question {
            return false;
        }
        let previous = self.selected_answer;
        match direction {
            Direction::Up => {
                if self.selected_answer >= 2 {
                    self.selected_answer -= 2;
                }
            }
            Direction::Down => {
                if self.selected_answer < 2 {
                    self.selected_answer += 2;
                }
            }
            Direction::Left => {
                if self.selected_answer % 2 == 1 {
                    self.selected_answer -= 1;
                }
            }
            Direction::Right => {
                if self.selected_answer % 2 == 0 {
                    self.selected_answer += 1;
                }
            }
        }
        previous != self.selected_answer
    }

    pub fn cycle_answer_selection(&mut self, detents: i32) -> bool {
        if self.state != FlagsState::Question || detents == 0 {
            return false;
        }
        self.selected_answer = wrap_index(
            self.selected_answer as i32 + detents,
            FLAGS_OPTION_COUNT as i32,
        ) as usize;
        true
    }

    pub fn confirm_answer(&mut self) {
        if self.state != FlagsState::Question {
            return;
        }
        self.last_answer_correct = self.selected_answer == self.correct_answer;
        if self.last_answer_correct {
            self.score += 1;
        }
        self.state = FlagsState::Feedback;
    }

    pub fn finish_feedback(&mut self, high_scores: &mut impl HighScoreStore, rng: &mut impl Rng) {
        if self.state != FlagsState::Feedback {
            return;
        }
        if self.active_mode == FlagsMode::DeathMatch && !self.last_answer_correct {
            high_scores.update_flags_death_match_best_score(self.score);
            self.refresh_best_score(high_scores);
            self.state = FlagsState::Over;
            return;
        }
        if self.active_mode == FlagsMode::Quiz20 && self.round >= FLAGS_QUIZ_ROUNDS {
            self.state = FlagsState::Results;
            return;
        }
        self.next_question(rng);
    }

    pub fn select_next_result_action(&mut self) -> bool {
        if self.state != FlagsState::Results && self.state != FlagsState::Over {
            return false;
        }
        self.result_action = match self.result_action {
            FlagsResultAction::Restart => FlagsResultAction::Exit,
            FlagsResultAction::Exit => FlagsResultAction::Restart,
        };
        true
    }

    pub fn select_previous_result_action(&mut self) -> bool {
        self.select_next_result_action()
    }

    pub fn cycle_result_action(&mut self, detents: i32) -> bool {
        if detents == 0 {
            false
        } else {
            self.select_next_result_action()
        }
    }

    pub const fn state(&self) -> FlagsState {
        self.state
    }

    pub const fn selected_mode(&self) -> FlagsMode {
        self.selected_mode
    }

    pub const fn active_mode(&self) -> FlagsMode {
        self.active_mode
    }

    pub const fn result_action(&self) -> FlagsResultAction {
        self.result_action
    }

    pub const fn pause_action(&self) -> FlagsPauseAction {
        self.pause_action
    }

    pub const fn choosing_action(&self) -> FlagsChoosingAction {
        self.choosing_action
    }

    pub const fn selected_answer(&self) -> usize {
        self.selected_answer
    }

    pub const fn correct_answer(&self) -> usize {
        self.correct_answer
    }

    pub const fn last_answer_correct(&self) -> bool {
        self.last_answer_correct
    }

    pub const fn answer_indices(&self) -> [usize; FLAGS_OPTION_COUNT] {
        self.answer_indices
    }

    pub const fn score(&self) -> u32 {
        self.score
    }

    pub const fn round(&self) -> u32 {
        self.round
    }

    pub const fn best_score(&self) -> u32 {
        self.best_score
    }

    pub fn current_flag(&self) -> FlagAsset {
        FLAG_ASSETS[self.correct_flag_index]
    }

    pub fn answer_flag(&self, index: usize) -> FlagAsset {
        FLAG_ASSETS[self.answer_indices[index]]
    }

    pub fn cycle_choosing_action_down(&mut self) -> bool {
        if self.state != FlagsState::ChoosingMode {
            return false;
        }
        self.choosing_action = match self.choosing_action {
            FlagsChoosingAction::Mode(FlagsMode::Practice) => {
                FlagsChoosingAction::Mode(FlagsMode::Quiz20)
            }
            FlagsChoosingAction::Mode(FlagsMode::Quiz20) => {
                FlagsChoosingAction::Mode(FlagsMode::DeathMatch)
            }
            FlagsChoosingAction::Mode(FlagsMode::DeathMatch) => FlagsChoosingAction::Exit,
            FlagsChoosingAction::Exit => FlagsChoosingAction::Mode(FlagsMode::Practice),
        };
        if let FlagsChoosingAction::Mode(mode) = self.choosing_action {
            self.selected_mode = mode;
        }
        true
    }

    pub fn cycle_choosing_action_up(&mut self) -> bool {
        if self.state != FlagsState::ChoosingMode {
            return false;
        }
        self.choosing_action = match self.choosing_action {
            FlagsChoosingAction::Mode(FlagsMode::Practice) => FlagsChoosingAction::Exit,
            FlagsChoosingAction::Mode(FlagsMode::Quiz20) => {
                FlagsChoosingAction::Mode(FlagsMode::Practice)
            }
            FlagsChoosingAction::Mode(FlagsMode::DeathMatch) => {
                FlagsChoosingAction::Mode(FlagsMode::Quiz20)
            }
            FlagsChoosingAction::Exit => {
                FlagsChoosingAction::Mode(FlagsMode::DeathMatch)
            }
        };
        if let FlagsChoosingAction::Mode(mode) = self.choosing_action {
            self.selected_mode = mode;
        }
        true
    }

    pub fn cycle_pause_action(&mut self) {
        self.pause_action = match self.pause_action {
            FlagsPauseAction::Continue => FlagsPauseAction::Exit,
            FlagsPauseAction::Exit => FlagsPauseAction::Continue,
        };
    }

    pub fn enter_paused(&mut self) {
        if self.state == FlagsState::Question || self.state == FlagsState::Feedback {
            self.state = FlagsState::Paused;
            self.pause_action = FlagsPauseAction::Continue;
        }
    }

    pub fn resume_from_pause(&mut self) {
        if self.state == FlagsState::Paused {
            self.state = FlagsState::Question;
        }
    }

    pub fn set_selected_mode(&mut self, mode: FlagsMode) {
        self.selected_mode = mode;
    }

    fn refresh_best_score(&mut self, high_scores: &impl HighScoreStore) {
        self.best_score = high_scores.flags_death_match_best_score();
    }

    fn reset_question_history(&mut self) {
        self.used_correct_flags.fill(false);
        self.used_correct_flag_count = 0;
    }

    fn next_question(&mut self, rng: &mut impl Rng) {
        self.state = FlagsState::Question;
        self.selected_answer = 0;
        self.round += 1;
        self.correct_flag_index = self.select_next_correct_flag(rng);
        self.correct_answer = rng.index(FLAGS_OPTION_COUNT);
        for i in 0..FLAGS_OPTION_COUNT {
            if i == self.correct_answer {
                self.answer_indices[i] = self.correct_flag_index;
            } else {
                loop {
                    let candidate = rng.index(self.flag_count);
                    if candidate != self.correct_flag_index
                        && !self.answer_indices[..i].contains(&candidate)
                    {
                        self.answer_indices[i] = candidate;
                        break;
                    }
                }
            }
        }
    }

    fn select_next_correct_flag(&mut self, rng: &mut impl Rng) -> usize {
        if self.used_correct_flag_count >= self.flag_count {
            self.reset_question_history();
        }
        let mut unused_offset = rng.index(self.flag_count - self.used_correct_flag_count);
        let mut selected = 0;
        for (index, used) in self.used_correct_flags.iter().enumerate() {
            if *used {
                continue;
            }
            if unused_offset == 0 {
                selected = index;
                break;
            }
            unused_offset -= 1;
        }
        self.used_correct_flags[selected] = true;
        self.used_correct_flag_count += 1;
        selected
    }
}

pub const fn flags_mode_name(mode: FlagsMode) -> &'static str {
    match mode {
        FlagsMode::Practice => "PRACTICE",
        FlagsMode::Quiz20 => "QUIZ 20",
        FlagsMode::DeathMatch => "DEATH MATCH",
    }
}

pub const fn flags_state_title(state: FlagsState) -> &'static str {
    match state {
        FlagsState::ChoosingMode | FlagsState::Question => "FLAGS",
        FlagsState::Feedback => "ANSWER",
        FlagsState::Paused => "PAUSED",
        FlagsState::Results => "RESULTS",
        FlagsState::Over => "GAME OVER",
    }
}

pub const fn flags_result_action_name(action: FlagsResultAction) -> &'static str {
    match action {
        FlagsResultAction::Restart => "RESTART",
        FlagsResultAction::Exit => "EXIT",
    }
}

const fn flags_mode_index(mode: FlagsMode) -> i32 {
    match mode {
        FlagsMode::Practice => 0,
        FlagsMode::Quiz20 => 1,
        FlagsMode::DeathMatch => 2,
    }
}

const fn flags_mode_from_index(index: i32) -> FlagsMode {
    match index {
        0 => FlagsMode::Practice,
        1 => FlagsMode::Quiz20,
        2 => FlagsMode::DeathMatch,
        _ => FlagsMode::Practice,
    }
}

pub const fn cycle_mode_from(mode: FlagsMode, detents: i32) -> FlagsMode {
    flags_mode_from_index(wrap_index(flags_mode_index(mode) + detents, 3))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::ScriptedRng;
    use crate::store::{HighScoreStore, MemoryHighScoreStore};

    #[test]
    fn question_has_four_unique_answers_and_correct_once() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);

        let mut answers = game.answer_indices().to_vec();
        answers.sort();
        answers.dedup();
        assert_eq!(answers.len(), FLAGS_OPTION_COUNT);
        assert_eq!(game.state(), FlagsState::Question);
        assert!(game.correct_answer() < FLAGS_OPTION_COUNT);
    }

    #[test]
    fn answer_navigation_matches_tile_grid() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);

        assert!(game.move_answer_selection(Direction::Right));
        assert_eq!(game.selected_answer(), 1);
        assert!(game.move_answer_selection(Direction::Down));
        assert_eq!(game.selected_answer(), 3);
        assert!(game.move_answer_selection(Direction::Left));
        assert_eq!(game.selected_answer(), 2);
    }

    #[test]
    fn death_match_wrong_answer_ends_and_persists_best() {
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::DeathMatch);
        game.start_selected_mode(&scores, &mut rng);
        game.cycle_answer_selection(1);
        game.confirm_answer();
        game.finish_feedback(&mut scores, &mut rng);

        assert_eq!(game.state(), FlagsState::Over);
        assert_eq!(scores.flags_death_match_best_score(), 0);
    }

    #[test]
    fn quiz20_finishes_after_twenty_rounds() {
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::Quiz20);
        game.start_selected_mode(&scores, &mut rng);
        for _ in 0..FLAGS_QUIZ_ROUNDS {
            game.confirm_answer();
            game.finish_feedback(&mut scores, &mut rng);
        }
        assert_eq!(game.round(), FLAGS_QUIZ_ROUNDS);
        assert_eq!(game.state(), FlagsState::Results);
    }

    #[test]
    fn cycle_mode_cycles_through_modes() {
        let mut game = FlagsGame::new(5);
        assert_eq!(game.selected_mode(), FlagsMode::Practice);
        assert!(game.cycle_mode(1));
        assert_eq!(game.selected_mode(), FlagsMode::Quiz20);
        assert!(game.cycle_mode(1));
        assert_eq!(game.selected_mode(), FlagsMode::DeathMatch);
        assert!(game.cycle_mode(1));
        assert_eq!(game.selected_mode(), FlagsMode::Practice);
    }

    #[test]
    fn cycle_mode_returns_false_when_no_change() {
        let mut game = FlagsGame::new(5);
        assert!(!game.cycle_mode(0));
    }

    #[test]
    fn cycle_mode_ignores_outside_choosing() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        assert!(!game.cycle_mode(1));
    }

    #[test]
    fn select_previous_and_next_mode() {
        let mut game = FlagsGame::new(5);
        assert!(game.select_next_mode());
        assert_eq!(game.selected_mode(), FlagsMode::Quiz20);
        assert!(game.select_previous_mode());
        assert_eq!(game.selected_mode(), FlagsMode::Practice);
    }

    #[test]
    fn move_answer_left_right_up_down() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        assert_eq!(game.selected_answer(), 0);

        assert!(game.move_answer_selection(Direction::Right));
        assert_eq!(game.selected_answer(), 1);
        assert!(!game.move_answer_selection(Direction::Right));
        assert_eq!(game.selected_answer(), 1);

        assert!(game.move_answer_selection(Direction::Down));
        assert_eq!(game.selected_answer(), 3);
        assert!(!game.move_answer_selection(Direction::Down));

        assert!(game.move_answer_selection(Direction::Left));
        assert_eq!(game.selected_answer(), 2);
        assert!(game.move_answer_selection(Direction::Up));
        assert_eq!(game.selected_answer(), 0);
    }

    #[test]
    fn move_answer_ignores_when_not_question() {
        let mut game = FlagsGame::new(5);
        assert!(!game.move_answer_selection(Direction::Right));
    }

    #[test]
    fn cycle_answer_selection() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        assert!(game.cycle_answer_selection(1));
        assert_eq!(game.selected_answer(), 1);
        assert!(game.cycle_answer_selection(1));
        assert_eq!(game.selected_answer(), 2);
        assert!(!game.cycle_answer_selection(0));
    }

    #[test]
    fn confirm_answer_correct_increments_score() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        game.confirm_answer();
        assert_eq!(game.state(), FlagsState::Feedback);
        assert!(game.last_answer_correct());
        assert_eq!(game.score(), 1);
    }

    #[test]
    fn confirm_answer_ignores_when_not_question() {
        let mut game = FlagsGame::new(5);
        game.confirm_answer();
        assert_eq!(game.state(), FlagsState::ChoosingMode);
    }

    #[test]
    fn finish_feedback_practice_goes_to_next_question() {
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4, 0, 1, 2, 3]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::Practice);
        game.start_selected_mode(&scores, &mut rng);
        game.confirm_answer();
        game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(game.state(), FlagsState::Question);
        assert_eq!(game.round(), 2);
    }

    #[test]
    fn finish_feedback_quiz20_mid_round_continues() {
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::Quiz20);
        game.start_selected_mode(&scores, &mut rng);
        game.confirm_answer();
        game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(game.state(), FlagsState::Question);
    }

    #[test]
    fn finish_feedback_ignores_when_not_feedback() {
        let mut game = FlagsGame::new(5);
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0]);
        game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(game.state(), FlagsState::ChoosingMode);
    }

    #[test]
    fn cycle_choosing_action_down_and_up() {
        let mut game = FlagsGame::new(5);
        assert_eq!(game.choosing_action(), FlagsChoosingAction::Mode(FlagsMode::Practice));
        assert!(game.cycle_choosing_action_down());
        assert_eq!(game.choosing_action(), FlagsChoosingAction::Mode(FlagsMode::Quiz20));
        assert!(game.cycle_choosing_action_down());
        assert_eq!(game.choosing_action(), FlagsChoosingAction::Mode(FlagsMode::DeathMatch));
        assert!(game.cycle_choosing_action_down());
        assert_eq!(game.choosing_action(), FlagsChoosingAction::Exit);
        assert!(game.cycle_choosing_action_down());
        assert_eq!(game.choosing_action(), FlagsChoosingAction::Mode(FlagsMode::Practice));

        assert!(game.cycle_choosing_action_up());
        assert_eq!(game.choosing_action(), FlagsChoosingAction::Exit);
    }

    #[test]
    fn cycle_choosing_action_ignores_outside_choosing_mode() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        assert!(!game.cycle_choosing_action_down());
    }

    #[test]
    fn cycle_pause_action_toggles() {
        let mut game = FlagsGame::new(5);
        assert_eq!(game.pause_action(), FlagsPauseAction::Continue);
        game.cycle_pause_action();
        assert_eq!(game.pause_action(), FlagsPauseAction::Exit);
        game.cycle_pause_action();
        assert_eq!(game.pause_action(), FlagsPauseAction::Continue);
    }

    #[test]
    fn enter_paused_only_from_question_or_feedback() {
        let mut game = FlagsGame::new(5);
        game.enter_paused();
        assert_eq!(game.state(), FlagsState::ChoosingMode);

        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        game.start_selected_mode(&scores, &mut rng);
        game.enter_paused();
        assert_eq!(game.state(), FlagsState::Paused);
    }

    #[test]
    fn resume_from_pause_only_when_paused() {
        let mut game = FlagsGame::new(5);
        game.resume_from_pause();
        assert_eq!(game.state(), FlagsState::ChoosingMode);
    }

    #[test]
    fn select_result_action_cycles() {
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::DeathMatch);
        game.start_selected_mode(&scores, &mut rng);
        game.cycle_answer_selection(1);
        game.confirm_answer();
        game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(game.state(), FlagsState::Over);
        assert!(game.select_next_result_action());
        assert_eq!(game.result_action(), FlagsResultAction::Exit);
        assert!(game.select_previous_result_action());
        assert_eq!(game.result_action(), FlagsResultAction::Restart);
    }

    #[test]
    fn cycle_result_action_cycles() {
        let mut scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 0, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::DeathMatch);
        game.start_selected_mode(&scores, &mut rng);
        game.cycle_answer_selection(1);
        game.confirm_answer();
        game.finish_feedback(&mut scores, &mut rng);
        assert_eq!(game.state(), FlagsState::Over);
        assert!(game.cycle_result_action(1));
        assert_eq!(game.result_action(), FlagsResultAction::Exit);
    }

    #[test]
    fn select_result_action_ignores_wrong_state() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.start_selected_mode(&scores, &mut rng);
        assert!(!game.select_next_result_action());
    }

    #[test]
    fn flags_mode_name_covers_all() {
        assert_eq!(flags_mode_name(FlagsMode::Practice), "PRACTICE");
        assert_eq!(flags_mode_name(FlagsMode::Quiz20), "QUIZ 20");
        assert_eq!(flags_mode_name(FlagsMode::DeathMatch), "DEATH MATCH");
    }

    #[test]
    fn flags_result_action_name_covers_all() {
        assert_eq!(flags_result_action_name(FlagsResultAction::Restart), "RESTART");
        assert_eq!(flags_result_action_name(FlagsResultAction::Exit), "EXIT");
    }

    #[test]
    fn flags_state_title_covers_all() {
        assert_eq!(flags_state_title(FlagsState::ChoosingMode), "FLAGS");
        assert_eq!(flags_state_title(FlagsState::Question), "FLAGS");
        assert_eq!(flags_state_title(FlagsState::Feedback), "ANSWER");
        assert_eq!(flags_state_title(FlagsState::Paused), "PAUSED");
        assert_eq!(flags_state_title(FlagsState::Results), "RESULTS");
        assert_eq!(flags_state_title(FlagsState::Over), "GAME OVER");
    }

    #[test]
    fn enter_choosing_resets_state() {
        let scores = MemoryHighScoreStore::new();
        let mut rng = ScriptedRng::new([0, 2, 1, 2, 3, 4]);
        let mut game = FlagsGame::new(5);
        game.set_selected_mode(FlagsMode::Practice);
        game.start_selected_mode(&scores, &mut rng);
        assert_eq!(game.state(), FlagsState::Question);
        game.enter_choosing(&scores);
        assert_eq!(game.state(), FlagsState::ChoosingMode);
        assert_eq!(game.result_action(), FlagsResultAction::Restart);
    }
}
