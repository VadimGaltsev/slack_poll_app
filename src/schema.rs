table! {
    channel_users (id) {
        id -> Int4,
        user_slack_id -> Nullable<Text>,
        user_thumbnail -> Nullable<Text>,
    }
}

table! {
    dialog_variants (id) {
        id -> Int4,
        day_id -> Int4,
        variant_text -> Text,
        max_score -> Int4,
    }
}

table! {
    poll (id) {
        id -> Int4,
        channel -> Text,
        is_closed -> Bool,
        time -> Nullable<Text>,
    }
}

table! {
    poll_variant (id) {
        id -> Int4,
        day_id -> Int4,
        title -> Nullable<Text>,
        variant -> Nullable<Text>,
        start_date -> Timestamp,
        end_date -> Nullable<Timestamp>,
    }
}

table! {
    votes_results (id) {
        id -> Int4,
        user_id -> Int4,
        day_id -> Int4,
        poll_variant_id -> Int4,
        dialog_variant_id -> Int4,
        score -> Int4,
    }
}

allow_tables_to_appear_in_same_query!(
    channel_users,
    dialog_variants,
    poll,
    poll_variant,
    votes_results,
);
