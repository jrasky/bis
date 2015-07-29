// Copyright 2015 Jerome Rasky <jerome@rasky.co>
//
// Licensed under the Apache License, version 2.0 (the "License"); you may not
// use this file except in compliance with the License. You may obtain a copy of
// the License at
//
//     <http://www.apache.org/licenses/LICENSE-2.0>
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either expressed or implied. See the
// License for the specific language concerning governing permissions and
// limitations under the License.

// all constants go in one namespace
pub const WHITESPACE_FACTOR: isize = 5;
pub const WHITESPACE_REDUCE: isize = 2;
pub const CLASS_FACTOR: isize = 3;
pub const FIRST_FACTOR: isize = 3;
pub const CLASS_REDUCE: isize = 2;

pub const DIST_WEIGHT: isize = -10;
pub const HEAT_WEIGHT: isize = 5;
pub const FACTOR_REDUCE: isize = 50;

pub const MAX_LEN: usize = 80;

pub const MATCH_NUMBER: usize = 10;

pub const EOT: char = '\u{4}';
pub const CTRL_U: char = '\u{15}';

pub const PROMPT: &'static str = "Match: ";
