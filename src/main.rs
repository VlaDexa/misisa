use alisa::Request;
use calamine::{open_workbook, open_workbook_auto, DataType, Range, Reader, Xls, Xlsx};
use itertools::Itertools;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    fmt::Display,
    fs::File,
    mem::MaybeUninit,
    net::Ipv4Addr,
    path::Path
};
use warp::{http::Response, Filter};

mod alisa;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
enum ClassType {
    Lection,
    Practice,
    Lab,
    Unknown(String),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Class {
    name: String,
    class_type: ClassType,
    teacher: Option<String>,
    room: String,
}

impl Class {
    fn new(name_and_teacher: &DataType, room: &DataType) -> Option<Self> {
        // Name and teacher in the first is placed in this way:
        // Name (Type)
        // Teacher?
        // The room is placed in a second cell

        let name_and_teacher = match name_and_teacher {
            DataType::String(s) => s,
            _ => return None,
        };

        let (name, class_type) = match name_and_teacher.split_once(" (") {
            Some((name, class_type)) => (name, class_type),
            None => return None,
        };

        let (class_type, mut teacher) = match class_type.split_once('\n') {
            Some((class, teacher)) => (class, Some(teacher)),
            None => (class_type, None),
        };

        if teacher.map(|teach| teach.is_empty()).unwrap_or_default() {
            teacher = None;
        }

        let class_type = match class_type.strip_suffix(')') {
            Some(class_type) => class_type,
            None => return None,
        };

        let class_type = match class_type {
            "Лекционные" => ClassType::Lection,
            "Практические" => ClassType::Practice,
            "Лабораторные" => ClassType::Lab,
            _ => ClassType::Unknown(class_type.to_string()),
        };

        let room = match room {
            DataType::String(s) => s,
            _ => return None,
        };

        Some(Self {
            name: name.to_string(),
            class_type,
            teacher: teacher.map(|s| s.to_string()),
            room: room.to_string(),
        })
    }
}

type Week = Box<[Day; 7]>;

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Subgroup {
    number: u8,
    days: Week,
}

#[derive(Debug, Serialize, Deserialize, Default, PartialEq, Clone)]
struct Day {
    upper_classes: [Option<Class>; 7],
    lower_classes: [Option<Class>; 7],
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
enum WeekInfo {
    WithSubgroups(Vec<Subgroup>),
    WithoutSubgroup(Week),
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct GroupInfo {
    name: String,
    subgroups: WeekInfo,
}

impl GroupInfo {
    fn get_subgroup(&self, subgroup_number: u8) -> Option<&Subgroup> {
        match &self.subgroups {
            WeekInfo::WithSubgroups(subgroups) => subgroups
                .iter()
                .find(|subgroup| subgroup.number == subgroup_number),
            WeekInfo::WithoutSubgroup(_) => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
struct Course {
    name: String,
    groups: Vec<GroupInfo>,
}

impl Course {
    fn new(name: String, groups: Vec<GroupInfo>) -> Self {
        Self { name, groups }
    }

    fn find_group(&self, group_name: &str) -> Option<&GroupInfo> {
        self.groups.iter().find(|group| group.name == group_name)
    }
}

struct ExcelData {
    pages: [(String, Range<DataType>); 4],
}

fn parse_schedules() -> std::io::Result<()> {
    // We have a dir for storing schedules
    // That dir has a "parsed" subdir and a "raw" subdir
    // For each file in the "raw" subdir we parse it and save it in the "parsed" subdir as json
    // The file names, not including file extensions, should match

    let raw_dir = Path::new("schedules").join("raw");
    let parsed_dir = Path::new("schedules").join("parsed");

    for entry in std::fs::read_dir(&raw_dir)? {
        let entry = entry?;
        let file_path = entry.path();
        assert!(file_path.is_file());
        let (file_name, extension) = (file_path.file_name(), file_path.extension());

        if std::fs::read_dir(&parsed_dir)?
            .map(|entry| entry.unwrap().path())
            .any(|x| x == file_path)
        {
            continue;
        }

        let excel = match extension {
            Some(ext) if ext == "xlsx" => {
                let mut excel_data: Xlsx<_> = open_workbook(&file_path).unwrap();
                ExcelData::new(&mut excel_data)
            }
            Some(ext) if ext == "xls" => {
                let mut excel_data: Xls<_> = open_workbook(&file_path).unwrap();
                ExcelData::new(&mut excel_data)
            }
            _ => {
                let mut excel_data = open_workbook_auto(&file_path).unwrap();
                ExcelData::new(&mut excel_data)
            }
        };

        let parsed = excel.parse();

        // Create a file with the same name as the original file
        let parsed_file_name = file_name.unwrap().to_str().unwrap();
        let parsed_file_path = parsed_dir.join(parsed_file_name).with_extension("json");
        let parsed_file = File::create(parsed_file_path)?;
        serde_json::to_writer_pretty(parsed_file, &parsed)?;
    }
    Ok(())
}

#[test]
fn test_excel_parsing() {
    use calamine::{open_workbook, Xlsx};
    let mut excel: Xlsx<_> = open_workbook("test/Test.xlsx").unwrap();
    let excel_data = ExcelData::new(&mut excel);
    let parsed = excel_data.parse();
    println!("Parsed: {:?}", parsed);

    let parsed_course = &parsed[0];
    let parsed_group = &parsed_course.groups[0];
    let (parsed_subgroup, second_parsed_subgroup) =
        if let WeekInfo::WithSubgroups(subgroups) = &parsed_group.subgroups {
            (&subgroups[0], &subgroups[1])
        } else {
            panic!("Expected subgroups, got {:?}", parsed_group.subgroups);
        };
    let parsed_day = &parsed_subgroup.days[0];
    let parsed_upper_class = parsed_day.upper_classes[0]
        .as_ref()
        .expect("Expected a class");
    let parsed_lower_class = second_parsed_subgroup.days[6].lower_classes[6]
        .as_ref()
        .expect("Expected a class");

    let test_upper_class = Class {
        name: String::from("Math"),
        class_type: ClassType::Practice,
        teacher: Some(String::from("Teacher")),
        room: String::from("Class"),
    };
    let test_lower_class = Class {
        name: String::from("CS"),
        class_type: ClassType::Lab,
        teacher: Some(String::from("Teacher2")),
        room: String::from("Class2"),
    };
    let test_day = Day {
        upper_classes: [
            Some(test_upper_class.clone()),
            None,
            None,
            None,
            None,
            None,
            None,
        ],
        lower_classes: Default::default(),
    };
    let test_second_day = Day {
        upper_classes: Default::default(),
        lower_classes: [
            None,
            None,
            None,
            None,
            None,
            None,
            Some(test_lower_class.clone()),
        ],
    };
    let test_subgroup = Subgroup {
        number: 1,
        days: Box::new([
            test_day.clone(),
            Day::default(),
            Day::default(),
            Day::default(),
            Day::default(),
            Day::default(),
            Day::default(),
        ]),
    };
    let test_second_subgroup = Subgroup {
        number: 2,
        days: Box::new([
            Day::default(),
            Day::default(),
            Day::default(),
            Day::default(),
            Day::default(),
            Day::default(),
            test_second_day,
        ]),
    };
    let test_group = GroupInfo {
        name: String::from("Group"),
        subgroups: WeekInfo::WithSubgroups(vec![test_subgroup.clone(), test_second_subgroup]),
    };
    let test_course = Course {
        name: String::from("Course"),
        groups: vec![test_group.clone()],
    };

    assert_eq!(parsed_upper_class, &test_upper_class);
    assert_eq!(parsed_lower_class, &test_lower_class);
    assert_eq!(parsed_day, &test_day);
    assert_eq!(parsed_subgroup, &test_subgroup);
    assert_eq!(parsed_group, &test_group);
    assert_eq!(parsed_course, &test_course);
}

impl ExcelData {
    fn new<T: std::io::Read + std::io::Seek>(sheets: &mut impl Reader<RS = T>) -> Self {
        let pages = sheets.sheet_names();
        assert_eq!(pages.len(), 4, "Excel file didn't have 4 pages");
        let (first, second, third, fourth) = if let [first, second, third, fourth] = pages {
            (first.clone(), second.clone(), third.clone(), fourth.clone())
        } else {
            unreachable!();
        };
        let info: [(String, Range<DataType>); 4] = [
            (sheets.worksheet_range(&first).unwrap().unwrap(), first).swap(),
            (sheets.worksheet_range(&second).unwrap().unwrap(), second).swap(),
            (sheets.worksheet_range(&third).unwrap().unwrap(), third).swap(),
            (sheets.worksheet_range(&fourth).unwrap().unwrap(), fourth).swap(),
        ];
        Self { pages: info }
    }

    fn parse(self) -> [Course; 4] {
        let mut courses: [MaybeUninit<Course>; 4] = [
            MaybeUninit::uninit(),
            MaybeUninit::uninit(),
            MaybeUninit::uninit(),
            MaybeUninit::uninit(),
        ];
        let courses_iter = self.pages.into_par_iter().map(|(name, sheet)| {
            let mut rows = sheet.rows();
            // This is a row with group names
            // We skip first 3 cells because info there doesn't matter
            // The only cells that matter are the ones with strings in them, so we skip the rest
            let first_row = rows
                .next()
                .unwrap()
                .iter()
                .skip(3)
                .filter(|cell| cell.is_string());
            // This is a row that contains info about subgroups
            // We skip first 3 cells because info there doesn't matter, same as the first one
            // Every second cell is guaranteed empty, so we skip it
            let second_row = rows.next().unwrap().iter().skip(3).step_by(2);
            // Capacity is 30, because in 2022 there were no more than 26 groups
            let mut subgroups: Vec<Option<Vec<u8>>> = Vec::with_capacity(30);
            /// Parses a cell into u8
            /// # Panics
            /// If contained data is not a string
            fn parse_datacell(cell: &DataType) -> u8 {
                cell.get_string().unwrap().parse().unwrap()
            }
            {
                // This is a vector that can contain numbers of subgroups in a group
                let mut subgroup_numbers: Option<Vec<u8>> = None;

                for (cell_num, cell) in second_row.enumerate() {
                    // If a cell is empty, it means that there is no subgroups in this group
                    // This means that we finished getting previous group's subgroups
                    // So we push already stored subgroups
                    // (But only if there were any)
                    if cell.is_empty() {
                        if cell_num != 0 {
                            subgroups.push(subgroup_numbers);
                        }
                        // if !subgroups.is_empty() {
                        //     subgroups.push(subgroup_numbers);
                        // }
                        subgroup_numbers = None;
                    } else {
                        assert!(cell.is_string());
                        if subgroup_numbers.is_none() {
                            // This means that we are at the start of a new group
                            // So we push None to subgroups to signalize that previous group hadn't subgroups
                            // (but only if it isn't the first group)
                            if cell_num != 0 {
                                subgroups.push(None);
                            }
                            subgroup_numbers = Some(Vec::with_capacity(3));
                        }
                        let subgroup_numbers_vec = subgroup_numbers.as_mut().unwrap();
                        // If the last element is higher than this one
                        // It means that we are at the start of a new group of subgroups
                        // push the previous vec to subgroups and create a new one with the first subgroup number
                        // Else we just continue adding numbers to the same vec
                        let parsed = parse_datacell(cell);
                        if subgroup_numbers_vec
                            .last()
                            .map(|last| last > &parsed)
                            .unwrap_or_default()
                        {
                            let mut new_vec = vec![parsed];
                            std::mem::swap(&mut new_vec, subgroup_numbers_vec);
                            subgroups.push(Some(new_vec));
                        } else {
                            subgroup_numbers_vec.push(parsed)
                        }
                        // subgroup_numbers = Some(subgroup_numbers_vec);
                    }
                }
                // Push the last subgroup numbers
                subgroups.push(subgroup_numbers)
            }

            let subgroups_num = subgroups
                .iter()
                .map(|el| el.as_ref().map(|el| el.len()).unwrap_or(1))
                .sum::<usize>();

            let mut classes: Vec<Week> = Vec::with_capacity(subgroups_num);

            for _ in 0..classes.capacity() {
                classes.push(Week::default());
            }

            for (row_count, (upper, lower)) in rows.tuple_windows().step_by(2).enumerate() {
                assert!(
                    row_count <= 7 * 7,
                    "Too many rows in a sheet, got {}",
                    row_count
                );
                // Monday is 0, Tuesday is 1, etc.
                let day_num = row_count / 7;
                assert!(day_num <= 6, "Too many days in a sheet, got {}", day_num);
                // First lesson is 0, second is 1, etc.
                let lesson_num = row_count % 7;
                assert!(
                    lesson_num <= 6,
                    "Too many lessons in a sheet, got {}",
                    lesson_num
                );

                assert_eq!(
                    (upper.len() - 3) / 2,
                    subgroups_num,
                    "Upper has wrong length, got {}, expected {}",
                    (upper.len() - 3) / 2,
                    subgroups_num
                );
                assert_eq!(
                    (lower.len() - 3) / 2,
                    subgroups_num,
                    "Lower has wrong length, got {}, expected {}",
                    (lower.len() - 3) / 2,
                    subgroups_num
                );

                let upper_iter = upper.iter().skip(3).tuple_windows().step_by(2);
                let lower_iter = lower.iter().skip(3).tuple_windows().step_by(2);
                for (
                    column_num,
                    ((name_and_teacher_upper, room_upper), (name_and_teacher_lower, room_lower)),
                ) in upper_iter.zip(lower_iter).enumerate()
                {
                    assert!(
                        column_num < subgroups_num,
                        "Too many columns in a sheet, got {}, expected max: {}",
                        column_num,
                        subgroups_num
                    );
                    let day = &mut classes[column_num][day_num];
                    let class_upper = Class::new(name_and_teacher_upper, room_upper);
                    let class_lower = Class::new(name_and_teacher_lower, room_lower);
                    day.upper_classes[lesson_num] = class_upper;
                    day.lower_classes[lesson_num] = class_lower;
                }
            }

            // // A vector that contains all the parsed days
            // let mut classes: Vec<Day> = Vec::with_capacity(subgroups_num * 7);
            // let classes_num = classes.capacity();

            // // Fill classes with default value
            // for _ in 0..classes_num {
            //     classes.push(Day::default());
            // }

            // for (mut class_number, (upper, lower)) in rows.tuple_windows().step_by(2).enumerate() {
            //     // Monday is 0, Tuesday is 1, etc.
            //     let day_number = class_number / 7;
            //     assert!(
            //         day_number < 7,
            //         "There are more than 7 days in a week, which is impossible"
            //     );
            //     class_number %= 7;
            //     for (
            //         ((name_and_teacher_upper, room_upper), (name_and_teacher_lower, room_lower)),
            //         day,
            //     ) in upper
            //         .iter()
            //         .skip(3)
            //         .tuple_windows()
            //         .zip(lower.iter().skip(3).tuple_windows())
            //         .zip(classes.iter_mut().skip(day_number * subgroups_num))
            //     {
            //         let class_upper = Class::new(name_and_teacher_upper, room_upper);
            //         let class_lower = Class::new(name_and_teacher_lower, room_lower);
            //         day.upper_classes[class_number] = class_upper;
            //         day.lower_classes[class_number] = class_lower;
            //     }
            // }

            //println!(
            //    "{}",
            //    simd_json::serde::to_string_pretty(classes.as_slice()).unwrap()
            //);

            // Classes is structured like [Monday * subgroups_num, Tuesday * subgroups_num, ...]
            // We need to split it into [Monday, Tuesday, ...] * subgroups_num
            // let mut thing: Vec<[Day; 7]> = Vec::with_capacity(subgroups_num);

            // for _ in 0..thing.capacity() {
            //     thing.push([
            //         Day::default(),
            //         Day::default(),
            //         Day::default(),
            //         Day::default(),
            //         Day::default(),
            //         Day::default(),
            //         Day::default(),
            //     ]);
            // }

            // let classes = classes.chunks(subgroups_num).enumerate().into_iter();

            // for (day, classes) in classes
            //     .into_iter()
            //     .chunks(subgroups_num)
            //     .into_iter()
            //     .enumerate()
            // {
            //     for (subgroup, class) in classes.into_iter().enumerate() {
            //         thing[subgroup][day] = class;
            //     }
            // }

            // assert_eq!(
            //     thing.len(),
            //     subgroups
            //         .iter()
            //         .map(|el| el.as_ref().map(|el| el.len()).unwrap_or(1))
            //         .sum::<usize>(),
            //     "Have {} subgroups, but {} weeks",
            //     subgroups_num,
            //     thing.len()
            // );
            let mut week_iter = classes.into_iter();

            let groups = first_row
                .zip(subgroups)
                .map(|(cell, subgroup)| {
                    if let DataType::String(name) = cell {
                        let name = name.clone();
                        if let Some(subgroups) = subgroup {
                            GroupInfo {
                                name,
                                subgroups: WeekInfo::WithSubgroups(
                                    subgroups
                                        .into_iter()
                                        .zip(&mut week_iter)
                                        .map(|(el, week)| Subgroup {
                                            number: el,
                                            days: week,
                                        })
                                        .collect(),
                                ),
                            }
                        } else {
                            GroupInfo {
                                name,
                                subgroups: WeekInfo::WithoutSubgroup(week_iter.next().unwrap()),
                            }
                        }
                    } else {
                        unreachable!()
                    }
                })
                .collect::<Vec<_>>();
            Course::new(name, groups)
        });

        courses_iter
            .zip(courses.par_iter_mut())
            .for_each(|(got, store)| {
                store.write(got);
            });

        // SAFETY: Just initialized it
        unsafe { courses.map(|el| el.assume_init()) }
    }
}

trait Swappable {
    type Output;

    fn swap(self) -> Self::Output;
}

impl<T1, T2> Swappable for (T1, T2) {
    type Output = (T2, T1);

    fn swap(self) -> Self::Output {
        let (a, b) = self;
        (b, a)
    }
}

impl Display for GroupInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.name)
    }
}

#[tokio::main]
async fn main() {
    parse_schedules().unwrap();
    let example1 = warp::get()
    .and(warp::path!("api" / "get_schedule"))
    .and(warp::query::<HashMap<String, String>>())
    .map(|p: HashMap<String, String>| match p.get("name") {
        Some(name) => Response::builder().body(format!("Hello, {}. This HTTP triggered function executed successfully.", name)),
        None => Response::builder().body(String::from("This HTTP triggered function executed successfully. Pass a name in the query string for a personalized response.")),
    });

    let includes_gzip = warp::header::exact_ignore_case("accept-encoding", "gzip, deflate, br");

    let path = Path::new("./schedules/parsed/itkn_31.08.json");
    let show_excel = warp::get()
        .and(warp::path!("api" / "get_excel"))
        .and(warp::fs::file(path));

    let show_excel_compressed = show_excel
        .clone()
        .and(includes_gzip)
        .with(warp::filters::compression::gzip());

    let show_bivt_21_15 = warp::get()
        .and(warp::path!("api" / "get_bivt_21_15"))
        .map(move || {
            // Open the file at the path
            let file = File::open(path).unwrap();
            // Read the json file
            let schedule: [Course; 4] = serde_json::from_reader(file).expect("Couldn't parse json");
            let course = &schedule[1];
            let group = course.find_group("БИВТ-21-15").unwrap();
            let subgroup = group.get_subgroup(1).unwrap();
            Response::builder()
                .header("Content-Type", "application/json")
                .body(serde_json::to_string(subgroup).unwrap())
        });

    let alisa_trigger = warp::get().and(warp::path!("api" / "alisa-trigger")).and(warp::body::json()).map(|input: Request| {
        dbg!(input);
        Response::builder().body("Ok")
    });

    let cert = warp::path!(".well-known").and(warp::fs::dir("./domain_ssl/.well-known"));

    let port_key = "FUNCTIONS_CUSTOMHANDLER_PORT";
    let port: u16 = match env::var(port_key) {
        Ok(val) => val.parse().expect("Custom Handler port is not a number!"),
        Err(_) => 3000,
    };

    let (_, warp) = warp::serve(
        example1
            .or(show_excel)
            .or(show_excel_compressed)
            .or(show_bivt_21_15)
            .or(cert)
            .or(alisa_trigger),
    )
    // .tls()
    // .cert_path("./domain_ssl/live/home.vladexa.rocks/fullchain.pem")
    // .key_path("./domain_ssl/live/home.vladexa.rocks/privkey.pem")
    .bind_ephemeral((Ipv4Addr::LOCALHOST, port));

    warp.await
}
