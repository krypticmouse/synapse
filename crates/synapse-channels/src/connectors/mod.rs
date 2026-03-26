macro_rules! define_connector {
    (
        $mod_name:ident,
        $struct_name:ident,
        $display_name:expr,
        $source_type:expr,
        [$($event:expr),* $(,)?]
    ) => {
        pub mod $mod_name {
            use async_trait::async_trait;
            use crate::{ChannelEvent, ChannelEventType, Connector, ConnectorConfig, ConnectorStatus};

            pub struct $struct_name {
                status: ConnectorStatus,
            }

            impl $struct_name {
                pub fn new() -> Self {
                    Self {
                        status: ConnectorStatus::Disconnected,
                    }
                }
            }

            #[async_trait]
            impl Connector for $struct_name {
                fn name(&self) -> &str {
                    $display_name
                }

                fn source_type(&self) -> &str {
                    $source_type
                }

                fn supported_events(&self) -> Vec<ChannelEventType> {
                    vec![$( ChannelEventType::from_str($event) ),*]
                }

                fn status(&self) -> ConnectorStatus {
                    self.status.clone()
                }

                async fn connect(&mut self, _config: &ConnectorConfig) -> anyhow::Result<()> {
                    self.status = ConnectorStatus::Connected;
                    tracing::info!(connector = $display_name, "connected");
                    Ok(())
                }

                async fn disconnect(&mut self) -> anyhow::Result<()> {
                    self.status = ConnectorStatus::Disconnected;
                    tracing::info!(connector = $display_name, "disconnected");
                    Ok(())
                }

                async fn poll(&self) -> anyhow::Result<Vec<ChannelEvent>> {
                    Ok(vec![])
                }
            }
        }
    };
}

// ═══════════════════════════════════════════════════════════════
// CHAT / MESSAGING PLATFORMS
// ═══════════════════════════════════════════════════════════════

define_connector!(
    telegram, TelegramConnector, "Telegram", "telegram",
    ["message", "edit", "delete", "reaction", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    discord, DiscordConnector, "Discord", "discord",
    ["message", "edit", "delete", "reaction", "thread_reply", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    slack, SlackConnector, "Slack", "slack",
    ["message", "edit", "delete", "reaction", "thread_reply", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    whatsapp, WhatsAppConnector, "WhatsApp", "whatsapp",
    ["message", "edit", "delete", "reaction", "file_upload"]
);

define_connector!(
    signal, SignalConnector, "Signal", "signal",
    ["message", "edit", "delete", "reaction", "file_upload"]
);

define_connector!(
    matrix, MatrixConnector, "Matrix", "matrix",
    ["message", "edit", "delete", "reaction", "thread_reply", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    email, EmailConnector, "Email", "email",
    ["message", "file_upload"]
);

define_connector!(
    teams, TeamsConnector, "Microsoft Teams", "teams",
    ["message", "edit", "delete", "reaction", "thread_reply", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    mattermost, MattermostConnector, "Mattermost", "mattermost",
    ["message", "edit", "delete", "reaction", "thread_reply", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    google_chat, GoogleChatConnector, "Google Chat", "google_chat",
    ["message", "edit", "delete", "reaction", "thread_reply", "member_join", "member_leave"]
);

define_connector!(
    webex, WebexConnector, "Webex", "webex",
    ["message", "edit", "delete", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    feishu, FeishuConnector, "Feishu/Lark", "feishu",
    ["message", "edit", "delete", "reaction", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    rocketchat, RocketChatConnector, "Rocket.Chat", "rocketchat",
    ["message", "edit", "delete", "reaction", "thread_reply", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    zulip, ZulipConnector, "Zulip", "zulip",
    ["message", "edit", "delete", "reaction", "thread_reply"]
);

define_connector!(
    xmpp, XmppConnector, "XMPP", "xmpp",
    ["message", "member_join", "member_leave"]
);

define_connector!(
    line, LineConnector, "LINE", "line",
    ["message", "file_upload", "member_join", "member_leave"]
);

define_connector!(
    viber, ViberConnector, "Viber", "viber",
    ["message", "file_upload"]
);

define_connector!(
    messenger, MessengerConnector, "Facebook Messenger", "messenger",
    ["message", "reaction", "file_upload"]
);

// ═══════════════════════════════════════════════════════════════
// SOCIAL PLATFORMS
// ═══════════════════════════════════════════════════════════════

define_connector!(
    mastodon, MastodonConnector, "Mastodon", "mastodon",
    ["message", "edit", "delete", "reaction"]
);

define_connector!(
    bluesky, BlueskyConnector, "Bluesky", "bluesky",
    ["message", "delete", "reaction"]
);

define_connector!(
    reddit, RedditConnector, "Reddit", "reddit",
    ["message", "edit", "delete", "reaction", "thread_reply"]
);

define_connector!(
    linkedin, LinkedInConnector, "LinkedIn", "linkedin",
    ["message", "reaction", "thread_reply"]
);

define_connector!(
    twitch, TwitchConnector, "Twitch", "twitch",
    ["message", "delete", "member_join", "member_leave"]
);

// ═══════════════════════════════════════════════════════════════
// DEVELOPER / COMMUNITY PLATFORMS
// ═══════════════════════════════════════════════════════════════

define_connector!(
    irc, IrcConnector, "IRC", "irc",
    ["message", "member_join", "member_leave"]
);

define_connector!(
    guilded, GuildedConnector, "Guilded", "guilded",
    ["message", "edit", "delete", "reaction", "thread_reply", "member_join", "member_leave"]
);

define_connector!(
    revolt, RevoltConnector, "Revolt", "revolt",
    ["message", "edit", "delete", "reaction", "member_join", "member_leave"]
);

define_connector!(
    keybase, KeybaseConnector, "Keybase", "keybase",
    ["message", "edit", "delete", "reaction"]
);

define_connector!(
    discourse, DiscourseConnector, "Discourse", "discourse",
    ["message", "edit", "delete", "reaction", "thread_reply"]
);

define_connector!(
    gitter, GitterConnector, "Gitter", "gitter",
    ["message", "edit", "delete"]
);

define_connector!(
    nextcloud_talk, NextcloudTalkConnector, "Nextcloud Talk", "nextcloud_talk",
    ["message", "edit", "delete", "reaction", "file_upload"]
);

// ═══════════════════════════════════════════════════════════════
// SECURE / ENCRYPTED MESSAGING
// ═══════════════════════════════════════════════════════════════

define_connector!(
    threema, ThreemaConnector, "Threema", "threema",
    ["message", "file_upload"]
);

define_connector!(
    nostr, NostrConnector, "Nostr", "nostr",
    ["message", "delete", "reaction"]
);

// ═══════════════════════════════════════════════════════════════
// VOICE / COLLABORATION
// ═══════════════════════════════════════════════════════════════

define_connector!(
    mumble, MumbleConnector, "Mumble", "mumble",
    ["message", "member_join", "member_leave"]
);

// ═══════════════════════════════════════════════════════════════
// ENTERPRISE / TEAM TOOLS
// ═══════════════════════════════════════════════════════════════

define_connector!(
    pumble, PumbleConnector, "Pumble", "pumble",
    ["message", "edit", "delete", "reaction", "thread_reply", "file_upload"]
);

define_connector!(
    flock, FlockConnector, "Flock", "flock",
    ["message", "file_upload"]
);

define_connector!(
    twist, TwistConnector, "Twist", "twist",
    ["message", "edit", "delete", "reaction", "thread_reply"]
);

define_connector!(
    dingtalk, DingTalkConnector, "DingTalk", "dingtalk",
    ["message", "file_upload", "member_join", "member_leave"]
);

// ═══════════════════════════════════════════════════════════════
// PUSH NOTIFICATIONS / WEBHOOKS
// ═══════════════════════════════════════════════════════════════

define_connector!(
    ntfy, NtfyConnector, "ntfy", "ntfy",
    ["message"]
);

define_connector!(
    gotify, GotifyConnector, "Gotify", "gotify",
    ["message"]
);

define_connector!(
    webhook, WebhookConnector, "Webhook", "webhook",
    ["message", "edit", "delete"]
);
