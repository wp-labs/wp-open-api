// 提供通配符匹配能力（用于测试与比较实现）。
// 使用 `*` 作为任意多字符通配符，例如：
//   "a*" 可匹配 "a"、"abc"。
/*
pub trait WildcardMatcher {
    fn matches(&self, other: &Self) -> bool;
}

impl WildcardMatcher for String {
    fn matches(&self, other: &Self) -> bool {
        wildmatch::WildMatch::new(self).matches(other)
    }
}
*/
