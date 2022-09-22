use bump_migrations::bumper::Migration;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Write};
use std::panic::set_hook;
use std::path::Path;
use std::{env, fs};

const INCORRECT_USAGE_MESSAGE: &str =
    r#"Incorrect usage of bump_migrations, please see bump_migrations --help for more details"#;
const HELP_MESSAGE: &str = r#"
Bump_migrations is a simple program that bumps django migrations in proper order so that merge migrations can be avoided.

USAGE:
    bump_migrations [OPTIONS] <dir_path> <migration_name> 

OPTIONS:
    <dir_path>                 Path to the django migration folder.
    <migration_name>           Name of the migration file to bump.

FLAGS:
    -h, --help             Print help information.
"#;

fn collect_migrations(path: &str) -> Vec<Migration> {
    let files = fs::read_dir(path).expect("Could not find directory.");

    let mut migrations: Vec<Migration> = vec![];
    for file in files {
        let file_name = String::from(file.unwrap().path().file_name().unwrap().to_str().unwrap());
        let migration_number = file_name.split("_").nth(0).unwrap().parse::<i32>();
        match migration_number {
            Ok(value) => migrations.push(Migration {
                number: value,
                name: file_name,
            }),
            Err(_) => println!("Not a migration file, carry on: ({})", file_name),
        };
    }

    migrations
}

fn update_dependency(
    path: &str,
    migrations: Vec<Migration>,
    migration_to_bump: Migration,
) -> Result<(), Box<dyn Error>> {
    let path = [String::from(path), migration_to_bump.name.to_owned()].join("");
    let file_path = Path::new(&path);
    let mut src = File::open(file_path)?;
    let mut contents = String::new();
    src.read_to_string(&mut contents)
        .expect("Unable to read the file");
    drop(src);

    let name_of_last_migration = match migrations.last() {
        Some(m) => m.name.to_owned(),
        None => return Err("No last migration found.".into()),
    };

    let idx_of_migration_to_bump = match migrations
        .iter()
        .position(|x| x.name == migration_to_bump.name)
    {
        Some(idx) => idx,
        None => return Err("Migration idx not found".into()),
    };

    let name_of_before_migration;
    if migrations[idx_of_migration_to_bump].number > migrations[idx_of_migration_to_bump - 1].number
    {
        name_of_before_migration = migrations[idx_of_migration_to_bump - 1].name.to_owned();
    } else {
        name_of_before_migration = migrations[idx_of_migration_to_bump - 2].name.to_owned();
    }

    let new_data = contents.replace(
        &*(name_of_before_migration.replace(".py", "")),
        &*(name_of_last_migration).replace(".py", ""),
    );

    let mut dst = File::create(&file_path)?;
    match dst.write(new_data.as_bytes()) {
        Ok(_) => Ok(()),
        Err(_) => Err("Could not write to file".into()),
    }
}

fn update_name(path: &str, name_before: &str, name_after: &str) -> Result<(), std::io::Error> {
    return fs::rename(
        format!("{}{}", path, name_before),
        format!("{}{}", path, name_after),
    );
}

fn bump_migration(path: &str, migration_name: &str) -> () {
    let mut migrations = collect_migrations(path);

    // Sort the migrations in ascending migration number order.
    migrations.sort_by(|a, b| a.number.cmp(&b.number));

    let migration_to_bump: Migration = Migration {
        name: String::from(migration_name),
        number: migration_name
            .split("_")
            .nth(0)
            .unwrap()
            .parse::<i32>()
            .unwrap(),
    };

    // Generate the new "bumped" name.
    let bumped_name = migration_to_bump.name.clone().replace(
        &migration_to_bump.number.to_string(),
        &(migrations.last().unwrap().number + 1).to_string(),
    );

    print!(
        "Bumping migration: {:?}   🤜 🤜 🤜 🤜 🤜 🤜 🤜 🤜 🤜 🤜   {:?}",
        migration_to_bump.name, bumped_name
    );

    let dependency_update = match update_dependency(path, migrations, migration_to_bump.to_owned())
    {
        Ok(_) => Ok(()),
        Err(e) => Err(e),
    };

    if dependency_update.is_err() {
        println!("{:?}", dependency_update);
    }

    let succesful_dependency_update = dependency_update.is_ok();

    if !succesful_dependency_update {
        println!("");
        println!("Failed to update dependency, terminating ... ❌");
        return;
    }

    let succesful_name_update = match update_name(path, &migration_to_bump.name, &bumped_name) {
        Ok(_) => true,
        Err(_) => false,
    };

    match succesful_dependency_update && succesful_name_update {
        true => println!(" ✅"),
        false => println!(" ❌"),
    };
}

fn bump(path: &str, migrations_to_bump: Vec<String>) -> () {
    for migration in migrations_to_bump {
        bump_migration(path, &migration[..]);
    }
}

fn main() {
    set_hook(Box::new(|info| {
        if let Some(s) = info.payload().downcast_ref::<String>() {
            println!("{}", s);
        }
    }));
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        println!("{}", INCORRECT_USAGE_MESSAGE);
        return;
    }
    let first_arg = &args[1];
    if first_arg == "-h" || first_arg == "--help" {
        println!("{}", HELP_MESSAGE);
    } else if args.len() >= 3 {
        let path = &args[1];
        let migrations_to_bump = args[2..].to_vec();
        bump(path, migrations_to_bump);
    } else {
        println!("{}", INCORRECT_USAGE_MESSAGE);
    }
}
