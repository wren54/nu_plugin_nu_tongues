/*
 POSIX locale string structure
   "en_xx.UTF_8@blank"
    ^   ^ ^---^ ^------^
    |   |   |      L modifier  - optional
    |   |   L encoding         - optional
    |   L country              - optional
    L language                 - mandatory



 Language File Structure

todo
*/

use lazy_static::lazy_static;
use nu_ansi_term::Color::{Fixed, Rgb};
use nu_ansi_term::*;
use nu_plugin::{serve_plugin, EvaluatedCall, LabeledError, MsgPackSerializer, Plugin};
use nu_protocol::{
    PluginSignature,  SyntaxShape, Type,
    Value as NuValue,
};
use regex::Regex;
use serde::Deserialize;
use std::env;
use std::fmt::{Display, Formatter, Result as FormatResult};
use std::fs::{read_dir, read_to_string};
use toml::{Table as TomlTable, Value as TomlValue};

const LOCALE_LANG: &str = "LANG";

struct Translate;

impl Translate {
    fn new() -> Self {
        Self
    }
}
//type Signature = PluginSignature;
impl Plugin for Translate {
    fn signature(&self) -> Vec<PluginSignature> {
        vec![PluginSignature::build("translate")
            .usage(
                "takes in a rosetta stone table as pipeline input and a msg_key as a string param",
            )
            .input_output_type(Type::String, Type::String)
            .required(
                "msg_key",
                SyntaxShape::String,
                "The name of the message in the translation files.",
            )
            .optional(
                "arguments",
                SyntaxShape::Record(Vec::<(String, SyntaxShape)>::new()),
                "The arguments for the translated string.",
            )]
    }

    fn run(
        &mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &NuValue,
    ) -> Result<NuValue, LabeledError> {
        //the call command must be "translate"
        assert_eq!(name, "translate");
        //gets the environmental variable $LANG and unwraps it
        let posix_lang_string: String = env::var(LOCALE_LANG).expect("no $LANG variable");
        let posix_lang: PosixLanguage =
            PosixLanguage::new(posix_lang_string).expect("$LANG was not in POSIX format");

        let mut path = input
            .as_string()
            .expect("input of translation was not String");
        if !path.ends_with("/") {
            path += "/";
        }
        //call.nth(0) returns the 1st positional parameter of translate command.
        //as per the .required function in signature, it is guaranteed to exist and be a String
        //Then pass it to the MessageKey object constructor
        let msg_key: MessageKey = MessageKey::new(
            call.nth(0)
                .expect("postitional param 0 of translate does not exist")
                .as_string()
                .expect("positional param 0 of translate was not a string"),
        );
        //takes in the dir input and searches all files in it for best translation file matching the user
        //reads it to String
        let best_file_path: String = posix_lang.get_best_file_path(path);

        let language_file_string: String = read_to_string(&best_file_path)
            .expect(format!("failed to open file at path {}", &best_file_path).as_str());
        //generates the translation from that file, reading the whole file
        //optimiztion here possibly
        let language_toml: LanguageToml = toml::from_str(language_file_string.as_str()).unwrap();
        /*let mut all_messages: String = "".to_string();

        for (key, val) in language_toml.messages.iter() {
            all_messages += &key;
            all_messages += &val.to_string();
        }*/

        //this TomlValue type allows the data to be treated as a table and a string simaltaneously
        let mut toml_value: TomlValue = toml::Value::Table(language_toml.messages);
        //loops through the path to get toml_value down to a String
        for key in msg_key.get_path().iter() {
            toml_value = toml_value.get(key).unwrap().to_owned();
        }

        //gets the string out
        let mut result: String = toml_value.to_string();

        //variable processing in our string
        let option = call.nth(1);
        if option.is_some() {
            let positionals = option.unwrap();
            for (arg, val) in positionals.as_record().unwrap().iter() {
                let parens = &("($".to_string() + &arg + ")").to_owned();
                result = result.replace(parens, val.as_string().unwrap().as_str());
            }
        }

        result = result.trim_start_matches('"').to_string();
        result = result.trim_end_matches('"').to_string();
        let ansi_result = ansify_string(&result);
        //ansi formatting

        //Color Processing
        /*
        let red_string = "red part";
        let green_string = "green part";

        let ansi_strings: AnsiStrings<'_> = AnsiStrings(&[Red.paint(red_string), Green.paint(green_string)]);

        */

        /*let msg_key_string = format!("{}", MessageKey::new(call.nth(0).map_or(  "error1".to_string() , |val| val.as_string().unwrap_or("error2".to_string()))));
        let posix_string = format!("{}", PosixLanguage::new(posix_lang.unwrap()).unwrap().to_string());
        let output = posix_string + &msg_key_string + &language_toml.language + &language_toml.territory + &language_toml.modifier + &all_messages;

        Ok(NuValue::string(output, input.span()))*/
        Ok(NuValue::string(ansi_result, input.span()))
    }
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct LanguageToml {
    language: String,
    territory: String,
    modifier: String,
    messages: TomlTable,
}


struct PosixLanguage {
    language: String,
    territory: String,
    encoding: String,
    modifier: String,
}

lazy_static! {
    pub static ref POSIX_LANG_CONSTRUCTION_REGEX: Regex = Regex::new( r"(?<language>[a-zA-Z]*)(?<territory>_..)?(?<encoding>\.(.*))?(?<modifier>\@([a-zA-Z0-9]*))?").unwrap();
    pub static ref MESSAGE_KEY_CONSTRUCTION_REGEX: Regex = Regex::new( r"(\.)" ).unwrap();
}

impl PosixLanguage {
    fn new(string: String) -> Option<Self> {
        let captures = match POSIX_LANG_CONSTRUCTION_REGEX.captures(&string) {
            Some(thing) => thing,
            None => return None,
        };
        Some(PosixLanguage {
            language: (&captures["language"]).to_string(),
            territory: (&captures)
                .name("territory")
                .map_or("_xx", |m| &m.as_str())
                .strip_prefix("_")
                .unwrap()
                .to_lowercase(),
            encoding: (&captures)
                .name("encoding")
                .map_or(".blank", |m| &m.as_str())
                .strip_prefix(".")
                .unwrap()
                .to_lowercase(),
            modifier: (&captures)
                .name("modifier")
                .map_or("@blank", |m| &m.as_str())
                .strip_prefix("@")
                .unwrap()
                .to_lowercase(),
        })
    }

    fn get_language(&self) -> &String {
        &self.language
    }
    fn get_territory(&self) -> &String {
        &self.territory
    }
    fn get_encoding(&self) -> &String {
        &self.encoding
    }
    fn get_modifier(&self) -> &String {
        &self.modifier
    }

    fn four_best_file_names(&self) -> Vec<String> {
        let mut file_names: Vec<String> = Vec::<String>::new();
        let territory: String = if self.get_territory() == "xx" {
            "".to_string()
        } else {
            "_".to_string() + &self.get_territory()
        };
        let modifier: String = if self.get_modifier() == "blank" {
            "".to_string()
        } else {
            "@".to_string() + &self.get_modifier()
        };

        file_names.push(self.get_language().to_owned() + &territory + &modifier + &".toml");
        file_names.push(self.get_language().to_owned() + &territory + &".toml");
        file_names.push(self.get_language().to_owned() + &modifier + &".toml");
        file_names.push(self.get_language().to_owned() + &".toml");
        file_names
    }

    fn get_best_file_path(&self, path: String) -> String {
        let four_best = self.four_best_file_names();

        for name in four_best.iter() {
            for option in read_dir(&path).expect(format!("there was no dir at {}", &path).as_str())
            {
                let dir = option.unwrap();
                if dir.file_type().unwrap().is_file() {
                    if dir.file_name() == name.as_str() {
                        return path + name;
                    }
                }
            }
        }
        for option in read_dir(&path).unwrap() {
            let dir = option.unwrap();
            match dir
                .file_name()
                .into_string()
                .unwrap()
                .strip_prefix(self.get_language())
            {
                Some(_) => return path + dir.file_name().to_str().unwrap(),
                None => todo!(),
            }
        }
        for option in read_dir(&path).unwrap() {
            let dir = option.unwrap();
            match dir.file_name().into_string().unwrap().strip_prefix("en") {
                Some(_) => return path + dir.file_name().to_str().unwrap(),
                None => todo!(),
            }
        }
        "failed_to_find_language_file".to_string()
    }
}

impl Display for PosixLanguage {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        write!(
            f,
            "lang: {}, terr: {}, encd: {}, mod: {}",
            self.get_language(),
            self.get_territory(),
            self.get_encoding(),
            self.get_modifier()
        )
    }
}

struct MessageKey {
    path: Vec<String>,
}

impl MessageKey {
    fn new(string: String) -> Self {
        let mut path: Vec<String> = Vec::new();
        for s_t_r in MESSAGE_KEY_CONSTRUCTION_REGEX.split(&string) {
            path.push(s_t_r.to_string());
        }
        MessageKey { path: path }
    }

    fn get_path(&self) -> &Vec<String> {
        &self.path
    }
}
impl Display for MessageKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        let mut output: String = "messagekey: ".to_string();
        for i in self.get_path().iter() {
            output += format!("{},", i).as_str();
        }
        output = output.strip_suffix(",").unwrap().to_string();
        write!(f, "{}", output)
    }
}
/*
impl CustomValue for AnsiStrings {
    fn value_string(&self) -> String {
        "AnsiStrings".to_string()
    }
    fn clone_value(&self, span: Span) -> Value {

    }
}*/

fn base_10_str_to_u8(string: &str) -> u8 {
    u8::from_str_radix(string, 10)
        .expect("string that was expected to be a u8 was formatted incorrectly")
}

fn ansify_string<'a>(input_string: &'a String) -> String {
    let input_string_split: Vec<&str> = input_string.split("(ansi ").collect();
    let mut ansi_result =
        Vec::<AnsiGenericString<'static, str>>::with_capacity(input_string_split.len());
    for i in 0..input_string_split.len() {
        ansi_result.push(if i == 0 {
            Color::Default.paint(input_string_split[i])
        } else {
            let seperate_command_from_rest: Vec<&str> =
                input_string_split[i].splitn(2, ")").collect();
            let command = seperate_command_from_rest[0];
            let the_rest = seperate_command_from_rest[1];

            let mut style = Style::new();

            if command.contains("bold") {
                style = style.bold()
            }
            if command.contains("dimmed") {
                style = style.dimmed()
            }
            if command.contains("italic") {
                style = style.italic()
            }
            if command.contains("underline") {
                style = style.underline()
            }
            if command.contains("strikethrough") {
                style = style.strikethrough()
            }
            if command.contains("hidden") {
                style = style.hidden()
            }
            if command.contains("blink") {
                style = style.blink()
            }
            
            style = ansi_compute_and_add_color(command, &style);

            if command.contains("reverse") {
                style = style.reverse()
            }

            style.paint(the_rest)
        });
    }

    let ansi_strings_copy: AnsiStrings<'a> = AnsiGenericStrings(ansi_result.leak());
    format!("{}", ansi_strings_copy)
}

fn ansi_compute_and_add_color(command: &str, style: &Style) -> Style {
    let mut result = style.clone();
    if command.contains("color") {
        let mut counter = 0;
        for (i, _match_word) in command.match_indices("color") {
            let mut color_args = command
                .get(
                    (i + 6)
                        ..(command.match_indices(']').collect::<Vec<(usize, &str)>>()[counter].0),
                )
                .unwrap()
                .trim_end_matches(']')
                .trim_start_matches('[');
            let is_foreground: bool = !color_args.starts_with("bg");

            color_args = color_args.trim_start_matches("bg;");
            color_args = color_args.trim_start_matches("fg;");

            let color_args_split: Vec<&str> = color_args.split(";").collect();

            let color = if color_args_split.len() < 3 {
                match ansi_color_from_str(color_args_split[0]) {
                    Some(color) => color,
                    None => Fixed(base_10_str_to_u8(color_args_split[0])),
                }
            } else {
                Rgb(
                    base_10_str_to_u8(color_args_split[0]),
                    base_10_str_to_u8(color_args_split[1]),
                    base_10_str_to_u8(color_args_split[2]),
                )
            };

            if is_foreground {
                result = style.fg(color);
            } else {
                result = style.on(color);
            }
            counter += 1;
        }
    }
    result
}

fn ansi_color_from_str(string: &str) -> Option<Color> {
    match string.trim().to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "blue" => Some(Color::Blue),
        "cyan" => Some(Color::Cyan),
        "darkgray" => Some(Color::DarkGray),
        "default" => Some(Color::Default),
        "green" => Some(Color::Green),
        "lightblue" => Some(Color::LightBlue),
        "lightcyan" => Some(Color::LightCyan),
        "lightgray" => Some(Color::LightGray),
        "lightgreen" => Some(Color::LightGreen),
        "lightmagenta" => Some(Color::LightMagenta),
        "lightpurple" => Some(Color::LightPurple),
        "lightred" => Some(Color::LightRed),
        "lightyellow" => Some(Color::LightYellow),
        "magenta" => Some(Color::Magenta),
        "purple" => Some(Color::Purple),
        "red" => Some(Color::Red),
        "white" => Some(Color::White),
        "yellow" => Some(Color::Yellow),
        _ => None,
    }
}

fn main() {
    serve_plugin(&mut Translate::new(), MsgPackSerializer);
}
