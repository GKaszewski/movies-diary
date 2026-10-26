pub struct PageItem {
    pub number: u32,
    pub is_current: bool,
    pub is_ellipsis: bool,
}

pub fn build_page_items(total_pages: u32, current_page: u32) -> Vec<PageItem> {
    if total_pages <= 1 {
        return vec![];
    }
    let mut set = std::collections::BTreeSet::new();
    set.insert(0u32);
    set.insert(total_pages - 1);
    let start = current_page.saturating_sub(2);
    let end = (current_page + 2).min(total_pages - 1);
    for p in start..=end {
        set.insert(p);
    }
    let pages: Vec<u32> = set.into_iter().collect();
    let mut items = Vec::new();
    for (i, &p) in pages.iter().enumerate() {
        if i > 0 && p > pages[i - 1] + 1 {
            items.push(PageItem {
                number: 0,
                is_current: false,
                is_ellipsis: true,
            });
        }
        items.push(PageItem {
            number: p,
            is_current: p == current_page,
            is_ellipsis: false,
        });
    }
    items
}
