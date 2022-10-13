#![warn(clippy::nursery, clippy::pedantic)]

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Year {
    year: Number,
    #[serde(default)]
    year_is_relative: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Month {
    month: Number,
    #[serde(default)]
    month_is_relative: bool, 
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Day {
    day: Number,
    #[serde(default)]
    day_is_relative: bool, 
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Hour{
    hour: Number,
    #[serde(default)]
    hour_is_relative: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Minute {
    minute: Number,
    #[serde(default)]
    minute_is_relative: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
struct DateTime {
    #[serde(flatten)]
    year: Option<Year>,
    #[serde(flatten)]
    month: Option<Month>,
    #[serde(flatten)]
    day: Option<Day>,
    #[serde(flatten)]
    hour: Option<Hour>,
    #[serde(flatten)]
    minute: Option<Minute>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
struct Fio {
    first_name: Option<String>,
    patronymic_name: Option<String>,
    last_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum Geolocation {
    Airport {
        airport: String
    },
    House {
        country: Option<String>,
        city: Option<String>, 
        street: Option<String>,
        house_number: Option<String>
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum YandexNumber {
    Integer(i64),
    Float(f64),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", content = "value")]
enum YandexEnteties {
    #[serde(rename = "YANDEX.FIO")]
    Fio(Fio),
    #[serde(rename = "YANDEX.GEO")]
    Geo(Geolocation),
    #[serde(rename = "YANDEX.DATETIME")]
    DateTime(DateTime),
    #[serde(rename = "YANDEX.NUMBER")]
    Number(YandexNumber),
}

#[test]
fn fio_deserialize() {
    use serde_json::json;
    let json = json!({
        "type": "YANDEX.FIO",
        "value": {
            "first_name": "антон",
            "patronymic_name": "павлович",
            "last_name": "чехов"
        }
    });
    let fio: YandexEnteties = serde_json::from_value(json).unwrap();
    assert_eq!(fio, YandexEnteties::Fio(Fio {
        first_name: Some("антон".to_string()),
        patronymic_name: Some("павлович".to_string()),
        last_name: Some("чехов".to_string()),
    }));
}

#[test]
fn number_float_deserialize() {
    use serde_json::json;
    let json = json!({
        "type": "YANDEX.NUMBER",
        "value": 4.5
    });
    let number: YandexEnteties = serde_json::from_value(json).unwrap();
    assert_eq!(number, YandexEnteties::Number(YandexNumber::Float(4.5)));
}

#[test]
fn number_integer_deserialize() {
    use serde_json::json;
    let json = json!({
        "type": "YANDEX.NUMBER",
        "value": 33
    });
    let number: YandexEnteties = serde_json::from_value(json).unwrap();
    assert_eq!(number, YandexEnteties::Number(YandexNumber::Integer(33)));
}

#[test]
fn geo_house_deserialize() {
    use serde_json::json;
    let json = json!({
        "type": "YANDEX.GEO",
        "value": {
            "country": "россия",
            "city": "москва",
            "street": "улица льва толстого",
            "house_number": "16"
        }
    });
    let geo = serde_json::from_value::<YandexEnteties>(json).unwrap();
    assert_eq!(geo, YandexEnteties::Geo(Geolocation::House {
        country: "россия".to_string().into(),
        city: "москва".to_string().into(),
        street: "улица льва толстого".to_string().into(),
        house_number: "16".to_string().into()
    }));
}

#[test]
fn geo_airpot_deserialize() {
    use serde_json::json;
    let json = json!({
        "type": "YANDEX.GEO",
        "value": {
            "airport": "аэропорт внуково",
        }
    });
    let geo = serde_json::from_value::<YandexEnteties>(json).unwrap();
    assert_eq!(geo, YandexEnteties::Geo(Geolocation::Airport {
        airport: "аэропорт внуково".to_string()
    }));
}

#[test]
fn date_time_deserialize() {
    use serde_json::json;
    let json = json!({
        "type": "YANDEX.DATETIME",
        "value": {
          "year": 1982,
          "month": 9,
          "day": -1,
          "day_is_relative": true,
        }
    });
    println!("{:?}", json);

    let dt = serde_json::from_value::<YandexEnteties>(json).unwrap();
    assert_eq!(
        dt,
        YandexEnteties::DateTime(DateTime {
            year: Some(Year { year: 1982.into(), year_is_relative: Default::default() }),
            month: Some(Month { month: 9.into(), month_is_relative: Default::default() }),
            day: Some(Day { day: (-1).into(), day_is_relative: true }),
            ..Default::default()
        })
    );
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
enum EntityValue {
    Number(Number),
    Object(Value),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
struct Token {
    /// First word of a named entity
    start: Number,
    /// First word after named entity
    end: Number
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Named Entities
struct Entity {
    /// Designation of the beginning and end of the named entity in the array of words.
    /// The numbering of words in the array starts from 0.
    tokens: Token,
    #[serde(flatten)]
    named_entity: YandexEnteties
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Words and entities which were extacted by Dialogs from user's request
struct Nlu {
    /// Words taken from user's phrase
    tokens: Vec<String>,
    /// Named entities
    entities: Vec<Entity>,
    /// Data extracted from user's request
    /// See [Natural language processing](https://yandex.ru/dev/dialogs/alice/doc/nlu.html)
    intents: Value,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[repr(transparent)]
/// The formal characteristics of the replica that Yandex Dialogs managed to highlight
struct Markup {
    /// A sign of a remark that contains criminal overtones (suicide, hate speech, threats). 
    /// You can set the skill to react in such cases, for example, to answer “I don’t understand what you mean. Please rephrase the question."
    ///
    /// Only `true` is possible. If the feature is not applicable, this property is not included in the response.
    dangerous_context: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
/// Input type
pub enum InputType {
    /// Voice input
    SimpleUtterance,
    /// Press of a button
    ButtonPressed,
    /// Audio player start event on smart speakers
    #[serde(rename = "AudioPlayer.PlaybackStarted")]
    PlaybackStarted,
    /// Playback end event
    #[serde(rename = "AudioPlayer.PlaybackFinished")]
    PlaybackFinished,
    /// An event about the imminent completion of the playback of the current track
    #[serde(rename = "AudioPlayer.PlaybackNearlyFinished")]
    PlaybackNearlyFinished,
    /// Playback stop
    #[serde(rename = "AudioPlayer.PlaybackStopped")]
    PlaybackStopped,
    /// Playback error
    #[serde(rename = "AudioPlayer.PlaybackFailed")]
    PlaybackFailed,
    /// Request for confirmation of payment in the skill
    #[serde(rename = "Purchase.Confirmation")]
    PurchaseConformation,
    /// Request to read data for the show
    #[serde(rename = "Show.Pull")]
    ShowPull,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// Data recieved from a user
pub struct Request {
    /// Normalized text of a request. During normalization text is cleaned up from punctuation marks,
    /// all the characters are converted to lowercase and numerals are converted to numbers
    /// When you run a skill on a request like "Ask the skill <Name> what time is it"
    /// in [`Request::command`], only the right part of the request will come: "what time is it".
    ///
    /// To get the exact request text, use the [`Request::original_utterance`] property.
    command: String,
    /// Full text of user request, max 1024 characters
    ///
    /// If the property contains the `"ping"` value,
    /// then the request is executed by Dialogs and is a test request.
    original_utterance: String,
    /// The formal characteristics of the replica that Yandex Dialogs managed to highlight.
    /// The property is missing if none of the nested properties are applicable.
    markup: Option<Markup>,
    /// The words and named entities that Dialogs retrieved from the user's query.
    nlu: Nlu,
    #[serde(rename = "type")]
    /// Input type.
    request_type: InputType,
}

#[test]
fn request_deserializes() {
    use serde_json::json;
    let json = json!({
          "command": "закажи пиццу на улицу льва толстого 16 на завтра",
          "original_utterance": "закажи пиццу на улицу льва толстого, 16 на завтра",
          "markup": {
            "dangerous_context": true
          },
          "payload": {},
          "nlu": {
            "tokens": [
              "закажи",
              "пиццу",
              "на",
              "льва",
              "толстого",
              "16",
              "на",
              "завтра"
            ],
            "entities": [
              {
                "tokens": {
                  "start": 2,
                  "end": 6
                },
                "type": "YANDEX.GEO",
                "value": {
                  "house_number": "16",
                  "street": "льва толстого"
                }
              },
              {
                "tokens": {
                  "start": 3,
                  "end": 5
                },
                "type": "YANDEX.FIO",
                "value": {
                  "first_name": "лев",
                  "last_name": "толстой"
                }
              },
              {
                "tokens": {
                  "start": 5,
                  "end": 6
                },
                "type": "YANDEX.NUMBER",
                "value": 16
              },
              {
                "tokens": {
                  "start": 6,
                  "end": 8
                },
                "type": "YANDEX.DATETIME",
                "value": {
                  "day": 1,
                  "day_is_relative": true
                }
              }
            ],
            "intents": {},
          },
          "type": "SimpleUtterance"
        });
        let json_string = serde_json::to_string_pretty(&json).unwrap();
        println!("{}", &json_string);
    let request: Request = serde_json::from_str(&json_string).unwrap();
    assert_eq!(request.command, "закажи пиццу на улицу льва толстого 16 на завтра");
    assert_eq!(request.original_utterance, "закажи пиццу на улицу льва толстого, 16 на завтра");
    assert_eq!(request.request_type, InputType::SimpleUtterance);
    assert!(request.markup.unwrap().dangerous_context);
    assert_eq!(&request.nlu.tokens, &["закажи", "пиццу", "на", "льва", "толстого", "16", "на", "завтра"]);
    assert_eq!(request.nlu.entities.len(), 4);
    assert_eq!(request.nlu.intents, json!({}));
}
