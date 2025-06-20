//! Leetcode data schemas
table! {
    problems(id) {
        category -> Text,
        fid -> Integer,
        id -> Integer,
        level -> Integer,
        locked -> Bool,
        name -> Text,
        percent -> Float,
        slug -> Text,
        starred -> Bool,
        status -> Text,
        desc -> Text,
        fstatus -> Text,
    }
}

// Tags
table! {
    tags(tag) {
        tag -> Text,
        refs -> Text,
    }
}
