// @generated automatically by Diesel CLI.

diesel::table! {
    documents (id) {
        id -> Nullable<Integer>,
        filename -> Text,
        path -> Text,
        summary -> Nullable<Text>,
        keywords -> Nullable<Text>,
        entities -> Nullable<Text>,
        topics -> Nullable<Text>,
        uploaded_at -> Nullable<Timestamp>,
    }
}

