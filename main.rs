/*
 POSIX locale string structure
   "en_xx.UTF_8@blank"
    ^   ^ ^---^ ^------^
    |   |   |      L modifier  - optional
    |   |   L encoding         - optional
    |   L country              - optional
    L language                 - mandatory
*/

 




use nu_plugin::{serve_plugin, MsgPackSerializer, Plugin, LabeledError, EvaluatedCall};
use nu_protocol::{Value as NuValue, Type, Signature, PluginSignature, SyntaxShape::Any as SyntaxShapeAny, SyntaxShape};
use regex::Regex;
use lazy_static::lazy_static;
use std::env;
use std::fmt::{Result as FormatResult, Formatter, Display};
use toml::{Table as TomlTable, Value as TomlValue,};
use serde::Deserialize;
use std::fs::{File, read_to_string, read_dir};
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
            .usage("takes in a rosetta stone table as pipeline input and a msg_key as a string param")
            .input_output_type(Type::String, Type::String )
            .required("msg_key", SyntaxShape::String, "The name of the message in the translation files.")
            .optional("arguments", SyntaxShape::Record(Vec::<(String,SyntaxShape)>::new()), "The arguments for the translated string.")
        ]
    }
    
    fn run(&mut self,
        name: &str,
        call: &EvaluatedCall,
        input: &NuValue
    ) -> Result<NuValue, LabeledError> {
        assert_eq!( name, "translate");
        let posix_lang_string:String = env::var(LOCALE_LANG).unwrap();
        let msg_key = MessageKey::new(call.nth(0).map_or(  "error1".to_string() , |val| val.as_string().unwrap_or("error2".to_string())));
        let posix_lang: PosixLanguage = PosixLanguage::new(posix_lang_string).unwrap();

        
        
        let language_file_string: String = read_to_string(posix_lang.get_best_file(input.as_string().unwrap())).unwrap();
        let language_toml: LanguageToml = toml::from_str(language_file_string.as_str()).unwrap();
        let mut all_messages: String = "".to_string();

        for (key,val) in language_toml.messages.iter() {
            all_messages += &key;
            all_messages += &val.to_string();
        }
        let mut toml_value: TomlValue = toml::Value::Table(language_toml.messages);
        for key in msg_key.get_path().iter(){
           toml_value =  toml_value.get(key).unwrap().to_owned(); 
        }
        let mut result = toml_value.to_string();

        let option = call.nth(1);
        if option.is_some() {
            let positionals = option.unwrap();
            for ( arg, val) in positionals.as_record().unwrap().iter() {
                let parens = &("($".to_string() + &arg + ")").to_owned();
                result = result.replace(parens, val.as_string().unwrap().as_str());
            }
        }
       /*let msg_key_string = format!("{}", MessageKey::new(call.nth(0).map_or(  "error1".to_string() , |val| val.as_string().unwrap_or("error2".to_string()))));
        let posix_string = format!("{}", PosixLanguage::new(posix_lang.unwrap()).unwrap().to_string());
        let output = posix_string + &msg_key_string + &language_toml.language + &language_toml.territory + &language_toml.modifier + &all_messages;
        
        Ok(NuValue::string(output, input.span()))*/
        Ok(NuValue::string(result, input.span()))
    } 

    
}

#[derive(Deserialize)]
struct LanguageToml{
    language: String,
    territory: String,
    modifier: String,
    messages: TomlTable
}





struct PosixLanguage {
    language: String,
    territory: String,
    encoding: String,
    modifier: String
}



lazy_static! {
    pub static ref POSIX_LANG_CONSTRUCTION_REGEX: Regex = Regex::new( r"(?<language>[a-zA-Z]*)(?<territory>_..)?(?<encoding>\.(.*))?(?<modifier>\@([a-zA-Z0-9]*))?").unwrap();
    pub static ref MESSAGE_KEY_CONSTRUCTION_REGEX: Regex = Regex::new( r"(\.)" ).unwrap();
}

impl PosixLanguage {
    fn new(string: String) -> Option<Self> {
        let captures = match POSIX_LANG_CONSTRUCTION_REGEX.captures(&string) {
            Some(thing) => thing,
            None => return None
        };
        Some(PosixLanguage {
            language: (&captures["language"]).to_string(),
            territory:(&captures).name("territory").map_or("_xx", |m| m.as_str()).strip_prefix("_").unwrap().to_string(),
            encoding: (&captures).name("encoding").map_or(".blank", |m| m.as_str()).strip_prefix(".").unwrap().to_string(),
            modifier: (&captures).name("modifier").map_or("@blank", |m| m.as_str()).strip_prefix("@").unwrap().to_string()
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
        let territory: String = if self.get_territory() == "xx" { "".to_string() } else { "_".to_string() + &self.get_territory()};
        let modifier: String = if self.get_modifier() == "blank" {"".to_string()} else {"@".to_string() + &self.get_modifier()};

        file_names.push(self.get_language().to_owned() + &territory + &modifier + &".toml" );
        file_names.push(self.get_language().to_owned() + &territory + &".toml");
        file_names.push(self.get_language().to_owned() + &modifier + &".toml");
        file_names.push(self.get_language().to_owned() + &".toml");
        file_names
    }

    fn get_best_file(&self, path: String) -> String {
        let four_best = self.four_best_file_names();
        
        for name in four_best.iter() {
            for option in read_dir(&path).unwrap() {
                let dir = option.unwrap();
                if dir.file_type().unwrap().is_file() {
                    if dir.file_name() == name.as_str() {
                        return path + name
                    }
                }
            }

        }
        for option in read_dir(&path).unwrap() {
            let dir = option.unwrap();
            match dir.file_name().into_string().unwrap().strip_prefix(self.get_language()) {
                Some(_) => return path + dir.file_name().to_str().unwrap(),
                None => todo!()

                
            }
        }
        for option in read_dir(&path).unwrap() {
            let dir = option.unwrap();
            match dir.file_name().into_string().unwrap().strip_prefix("en") {
                Some(_) => return path + dir.file_name().to_str().unwrap(),
                None => todo!()
               
            }
        }
        "failed_to_find_language_file".to_string()
    }
}

impl Display for PosixLanguage {
    fn fmt (&self, f: &mut Formatter<'_>) -> FormatResult {
        write!(f, "lang: {}, terr: {}, encd: {}, mod: {}", self.get_language(), self.get_territory(), self.get_encoding(), self.get_modifier())
    }
}


struct MessageKey {
    path: Vec<String>
}

impl MessageKey {
    fn new( string: String) -> Self {
        let mut path: Vec<String> = Vec::new();
        for s_t_r in  MESSAGE_KEY_CONSTRUCTION_REGEX.split(&string) {
            path.push(s_t_r.to_string());
        }
        MessageKey {
            path: path
        }
    }

    fn get_path(&self) -> &Vec<String> {
        &self.path
    }
}
impl Display for MessageKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> FormatResult {
        let mut output: String = "messagekey: ".to_string();
        for i in self.get_path().iter() {
            output += format!( "{},", i).as_str(); 
        } 
        output = output.strip_suffix(",").unwrap().to_string();
        write!(f, "{}", output)
    }
}

fn main() {
    serve_plugin(&mut Translate::new(), MsgPackSerializer);
}
