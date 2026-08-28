use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{BitAnd, BitOr, BitOrAssign, Not};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Permissions(pub u64);

macro_rules! perms {
    ($( $name:ident = $bit:expr ),* $(,)?) => {
        impl Permissions {
            $( pub const $name: Permissions = Permissions(1 << $bit); )*
            pub const ALL_NAMED: &'static [(&'static str, Permissions)] = &[ $( (stringify!($name), Permissions::$name) ),* ];
        }
    };
}

perms! {
    VIEW_CHANNEL = 0,
    SEND_MESSAGES = 1,
    EMBED_LINKS = 2,
    ATTACH_FILES = 3,
    ADD_REACTIONS = 4,
    USE_EXTERNAL_EMOJI = 5,
    MENTION_EVERYONE = 6,
    READ_HISTORY = 7,
    SEND_TTS = 8,
    BYPASS_SLOWMODE = 9,

    MANAGE_MESSAGES = 16,
    PIN_MESSAGES = 17,
    MANAGE_CHANNELS = 18,
    MANAGE_ROLES = 19,
    MANAGE_EMOJI = 20,
    MANAGE_WEBHOOKS = 21,
    MANAGE_GUILD = 22,
    VIEW_AUDIT_LOG = 23,
    CREATE_INVITES = 24,
    CHANGE_NICKNAME = 25,
    MANAGE_NICKNAMES = 26,
    VIEW_CHANNEL_MEMBERS = 27,

    KICK_MEMBERS = 32,
    BAN_MEMBERS = 33,
    MODERATE_MEMBERS = 34,

    CONNECT = 40,
    SPEAK = 41,
    STREAM = 42,
    VIDEO = 43,
    USE_VAD = 44,
    PRIORITY_SPEAKER = 45,
    MUTE_MEMBERS = 46,
    DEAFEN_MEMBERS = 47,
    MOVE_MEMBERS = 48,
    USE_SOUNDBOARD = 49,

    ADMINISTRATOR = 63,
}

impl Permissions {
    pub const NONE: Permissions = Permissions(0);

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub const fn contains(self, other: Permissions) -> bool {
        self.0 & other.0 == other.0
    }

    pub const fn intersects(self, other: Permissions) -> bool {
        self.0 & other.0 != 0
    }

    pub const fn union(self, other: Permissions) -> Permissions {
        Permissions(self.0 | other.0)
    }

    pub const fn difference(self, other: Permissions) -> Permissions {
        Permissions(self.0 & !other.0)
    }

    pub fn is_admin(self) -> bool {
        self.contains(Self::ADMINISTRATOR)
    }

    pub fn default_everyone() -> Permissions {
        Self::VIEW_CHANNEL
            | Self::SEND_MESSAGES
            | Self::EMBED_LINKS
            | Self::ATTACH_FILES
            | Self::ADD_REACTIONS
            | Self::USE_EXTERNAL_EMOJI
            | Self::READ_HISTORY
            | Self::CREATE_INVITES
            | Self::CHANGE_NICKNAME
            | Self::VIEW_CHANNEL_MEMBERS
            | Self::CONNECT
            | Self::SPEAK
            | Self::STREAM
            | Self::VIDEO
            | Self::USE_VAD
            | Self::USE_SOUNDBOARD
    }

    pub fn names(self) -> Vec<&'static str> {
        Self::ALL_NAMED
            .iter()
            .filter(|(_, p)| self.contains(*p))
            .map(|(n, _)| *n)
            .collect()
    }
}

impl BitOr for Permissions {
    type Output = Permissions;
    fn bitor(self, rhs: Self) -> Self {
        Permissions(self.0 | rhs.0)
    }
}

impl BitOrAssign for Permissions {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Permissions {
    type Output = Permissions;
    fn bitand(self, rhs: Self) -> Self {
        Permissions(self.0 & rhs.0)
    }
}

impl Not for Permissions {
    type Output = Permissions;
    fn not(self) -> Self {
        Permissions(!self.0)
    }
}

impl fmt::Debug for Permissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Permissions({:?})", self.names())
    }
}

impl Serialize for Permissions {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse::<u64>().map(Permissions).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overwrite {
    pub allow: Permissions,
    pub deny: Permissions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Membership {
    NotMember,
    Member,
    Owner,
    InstanceAdmin,
}

pub struct PermissionInputs<'a> {
    pub membership: Membership,
    pub everyone: Permissions,
    pub roles: &'a [Permissions],
    pub category_everyone: Option<Overwrite>,
    pub category_roles: &'a [Overwrite],
    pub category_member: Option<Overwrite>,
    pub channel_everyone: Option<Overwrite>,
    pub channel_roles: &'a [Overwrite],
    pub channel_member: Option<Overwrite>,
}

pub fn resolve(inputs: &PermissionInputs<'_>) -> Permissions {
    match inputs.membership {
        Membership::NotMember => return Permissions::NONE,
        Membership::Owner | Membership::InstanceAdmin => return Permissions(u64::MAX),
        Membership::Member => {}
    }
    let mut base = inputs.everyone;
    for r in inputs.roles {
        base |= *r;
    }
    if base.is_admin() {
        return Permissions(u64::MAX);
    }
    let mut p = base;
    p = apply_tier(p, inputs.category_everyone, inputs.category_roles, inputs.category_member);
    p = apply_tier(p, inputs.channel_everyone, inputs.channel_roles, inputs.channel_member);
    p
}

fn apply_tier(
    mut p: Permissions,
    everyone: Option<Overwrite>,
    roles: &[Overwrite],
    member: Option<Overwrite>,
) -> Permissions {
    if let Some(o) = everyone {
        p = p.difference(o.deny).union(o.allow);
    }
    let mut allow = Permissions::NONE;
    let mut deny = Permissions::NONE;
    for o in roles {
        allow |= o.allow;
        deny |= o.deny;
    }
    p = p.difference(deny).union(allow);
    if let Some(o) = member {
        p = p.difference(o.deny).union(o.allow);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base<'a>(membership: Membership, roles: &'a [Permissions]) -> PermissionInputs<'a> {
        PermissionInputs {
            membership,
            everyone: Permissions::default_everyone(),
            roles,
            category_everyone: None,
            category_roles: &[],
            category_member: None,
            channel_everyone: None,
            channel_roles: &[],
            channel_member: None,
        }
    }

    #[test]
    fn non_member_gets_nothing_even_with_channel_allow() {
        let mut i = base(Membership::NotMember, &[]);
        i.channel_everyone = Some(Overwrite { allow: Permissions::VIEW_CHANNEL, deny: Permissions::NONE });
        assert_eq!(resolve(&i), Permissions::NONE);
    }

    #[test]
    fn owner_has_everything() {
        assert!(resolve(&base(Membership::Owner, &[])).contains(Permissions::MANAGE_GUILD));
    }

    #[test]
    fn channel_member_overwrite_beats_role_deny() {
        let mut i = base(Membership::Member, &[]);
        i.channel_roles = &[Overwrite { allow: Permissions::NONE, deny: Permissions::SEND_MESSAGES }];
        i.channel_member = Some(Overwrite { allow: Permissions::SEND_MESSAGES, deny: Permissions::NONE });
        assert!(resolve(&i).contains(Permissions::SEND_MESSAGES));
    }

    #[test]
    fn channel_tier_overrides_category_tier() {
        let mut i = base(Membership::Member, &[]);
        i.category_everyone = Some(Overwrite { allow: Permissions::NONE, deny: Permissions::VIEW_CHANNEL });
        i.channel_everyone = Some(Overwrite { allow: Permissions::VIEW_CHANNEL, deny: Permissions::NONE });
        assert!(resolve(&i).contains(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn admin_role_ignores_overwrites() {
        let roles = [Permissions::ADMINISTRATOR];
        let mut i = base(Membership::Member, &roles);
        i.channel_everyone = Some(Overwrite { allow: Permissions::NONE, deny: Permissions::VIEW_CHANNEL });
        assert!(resolve(&i).contains(Permissions::VIEW_CHANNEL));
    }

    #[test]
    fn serializes_as_decimal_string() {
        let p = Permissions::VIEW_CHANNEL | Permissions::SPEAK;
        let s = serde_json::to_string(&p).unwrap();
        let back: Permissions = serde_json::from_str(&s).unwrap();
        assert_eq!(back, p);
    }
}
