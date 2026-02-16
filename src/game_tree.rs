use bevy::ecs::event::EventWriter;
use bevy_egui::egui::{self, response, text::Fonts, Align2, FontId, Grid, Label, RichText, Sense};
use bevy::prelude::warn;
use crate::{
    board::*, event::DeleteVariationEvent, fen::{read_fen, write_fen, INITIAL_FEN}, pgn::parse_pgn, piece::PieceColor, step::{read_step, write_step}, UpdateBoard
};

#[derive(Clone)]
struct MoveData {
    ply: usize,     // 步数编号(1,2,3...)
    san: String,    // 标准代数记法
    color: PieceColor,  // 行动方的颜色
}

#[derive(Clone)]
struct GameTreeNode {
    board: Board,
    sons: Vec<(Step, usize, MoveData)>, // 默认第一个是主分支
    parent: Option<usize>,
}

impl GameTreeNode {
    fn new(board: Board) -> Self {
        GameTreeNode {
            board: board,
            sons: Vec::new(),
            parent: None,
        }
    }
}

#[derive(Default)]
pub struct GameTree {
    nodes: Vec<GameTreeNode>,
    root: usize,
    focus: usize,
}

impl GameTree {
    pub fn new(board: Board) -> Self {
        GameTree {
            nodes: vec![GameTreeNode::new(board)],
            root: 0,
            focus: 0,
        }
    }

    pub fn from_pgn(pgn: String) -> Self {
        let mut board = read_fen(INITIAL_FEN.to_string());
        let mut tree = Self::new(board.clone());

        let steps = parse_pgn(&pgn);

        for step in steps.iter() {
            if let Some(s) = read_step(&board, step.clone()) {
                if tree.try_move(s) {
                    board = tree.board();
                }
            }
        }

        tree
    }

    pub fn pgn(&self, mut current: usize) -> String {
        let mut path: Vec<usize> = Vec::new();
        let mut sans: Vec<String> = Vec::new();

        while current != self.root {
            if let Some(parent) = self.nodes[current].parent {
                path.push(current);
                current = parent;
            } else {
                unreachable!()
            }
        }
        path.reverse();

        current = self.root;
        for node in path {
            for (_step, son, move_data) in self.nodes[current].sons.iter() {
                if node == *son {
                    match move_data.color {
                        PieceColor::White => sans.push(format!("{}.{}", move_data.ply, move_data.san)),
                        PieceColor::Black => sans.push(move_data.san.clone()),
                    }
                    current = *son;
                    break
                }
            }
        }
        sans.join(" ")
    }

    pub fn from_string(s: String) -> Option<Self> {
        let lines: Vec<&str> = s.trim().lines().collect();
    
        if lines.len() < 4 || lines[0] != "[chess game tree]" {
            return None
        }

        let initial_fen = lines[1];
        let nodes_count: usize = lines[2].parse().unwrap_or(0);
        let info_lines = &lines[3..];

        let mut tree = GameTree {
            nodes: vec![GameTreeNode::new(read_fen(initial_fen.to_string())); nodes_count],
            root: 0,
            focus: 0,
        };

        for (node_id, line) in info_lines.iter().enumerate() {
            if node_id >= nodes_count {
                break 
            }

            let move_infos: Vec<&str> = line.split('|').collect();

            for move_info in move_infos {
                // 解析 (son_id, san)
                if let Some(inner) = move_info.strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')')) 
                {
                    let parts: Vec<&str> = inner.splitn(2, ", ").collect();
                    if parts.len() == 2 {
                        let Ok(son_id) = parts[0].parse::<usize>() else {
                            return None
                        };
                        let san = parts[1].to_string();
                        let Some(step) = read_step(&tree.nodes[node_id].board, san) else {
                            return None
                        };
                        let Some(board) = try_move(&tree.nodes[node_id].board, step) else {
                            return None 
                        };
                        tree.nodes[son_id].board = board;
                        tree.nodes[son_id].parent = Some(node_id);
                        let move_data = MoveData {
                            ply: tree.nodes[node_id].board.fullmove,
                            san: write_step(&tree.nodes[node_id].board, step).unwrap(),
                            color: tree.nodes[node_id].board.active_color,
                        };
                        tree.nodes[node_id].sons.push((step, son_id, move_data));
                    }
                }
            }
        }

        Some(tree) 
    }

    pub fn to_string(&self) -> String {
        let title = "[chess game tree]";
        let initial = write_fen(self.nodes[self.root].board.clone());
        let nodes = self.nodes.len();

        let info = self.nodes.iter().map(|node| {
            node.sons.iter().map(|(_step, son, move_data)| {
                format!("({}, {})", son, move_data.san)
            })
            .collect::<Vec<String>>()
            .join("|")
        })
        .collect::<Vec<String>>()
        .join("\n");

        format!("{}\n{}\n{}\n{}", title, initial, nodes, info)
    }

    pub fn board(&self) -> Board {
        self.nodes[self.focus].board.clone()
    }

    pub fn is_first_board(&self) -> bool {
        return self.focus == self.root
    }

    pub fn is_last_board(&self) -> bool {
        return self.nodes[self.focus].sons.len() == 0
    }

    pub fn move_to_start(&mut self) {
        while let Some(parent) = self.nodes[self.focus].parent {
            self.focus = parent;
        }
    }

    pub fn move_backward(&mut self) {
        if let Some(parent) = self.nodes[self.focus].parent {
            self.focus = parent;
        }
    }

    pub fn move_forward(&mut self) {
        if !self.nodes[self.focus].sons.is_empty() {
            self.focus = self.nodes[self.focus].sons[0].1;
        }
    }

    pub fn move_to_end(&mut self) {
        while !self.nodes[self.focus].sons.is_empty() {
            self.focus = self.nodes[self.focus].sons[0].1;
        }
    }

    fn move_to_node(&mut self, idx: usize) {
        self.focus = idx;
    }

    fn move_to_node_response(&mut self, idx: usize, mut event_writer: EventWriter<UpdateBoard>) -> impl FnOnce() {
        move || {
            self.move_to_node(idx);
            event_writer.write(UpdateBoard {
                new_board: self.board()
            });
        }
    }

    // 由于rust的禁止双重借用的规则被迫用了比较奇怪的写法，实际上函数式会好一些
    pub fn try_move(&mut self, step: Step) -> bool {
        {
            let node = &self.nodes[self.focus];
            for (s, son, _) in &node.sons {
                if step == *s {
                    self.focus = *son;
                    return true;
                }
            }
        }
        if let Some(board) = try_move(&self.nodes[self.focus].board, step) {
            let new_index = self.nodes.len();
            self.nodes.push(GameTreeNode::new(board));
            self.nodes[new_index].parent = Some(self.focus);
            let move_data = MoveData {
                ply: self.nodes[self.focus].board.fullmove,
                san: write_step(&self.nodes[self.focus].board, step).unwrap(),
                color: self.nodes[self.focus].board.active_color,
            };
            self.nodes[self.focus].sons.push((step, new_index, move_data));
            self.focus = new_index;
            true
        } else {
            false
        }
    }

    fn collect_remaining_nodes(
        &mut self, 
        current: usize, 
        node_to_delete: usize, 
        remaining_nodes: &mut Vec<usize>,
        node_mapping: &mut Vec<Option<usize>>,
    ) {
        if current == node_to_delete {
            return;
        }

        let new_index = remaining_nodes.len();
        remaining_nodes.push(current);
        node_mapping[current] = Some(new_index);

        for (_, son, _) in self.nodes[current].sons.clone() {
            self.collect_remaining_nodes(son, node_to_delete, remaining_nodes, node_mapping);
        }
    }

    pub fn handle_delete_variation(&mut self, e: &DeleteVariationEvent, event_writer: &mut EventWriter<UpdateBoard>) {
        let current = e.node_to_delete;
        if current == self.root {
            warn!("Try to delete game tree root");
            return;
        }

        let mut remaining_nodes = Vec::new();
        let mut node_mapping = vec![None; self.nodes.len()];

        self.collect_remaining_nodes(self.root, current, &mut remaining_nodes, &mut node_mapping);

        // 创建新的节点向量并更新索引
        let mut new_nodes = Vec::with_capacity(remaining_nodes.len());
        
        // 首先创建所有节点（但sons和parent还未更新）
        for &old_index in &remaining_nodes {
            let mut node = self.nodes[old_index].clone();
            node.sons.clear(); // 清空子节点，稍后重新建立
            node.parent = node.parent.and_then(|p| node_mapping[p]); // 更新父节点索引
            new_nodes.push(node);
        }
        
        // 然后重新建立子节点关系
        for &old_index in &remaining_nodes {
            let new_index = node_mapping[old_index].unwrap();
            for (step, son_old_index, move_data) in &self.nodes[old_index].sons {
                if let Some(son_new_index) = node_mapping[*son_old_index] {
                    new_nodes[new_index].sons.push((*step, son_new_index, move_data.clone()));
                }
            }
        }

        self.nodes = new_nodes;
        self.root = 0;
        // 更新焦点。如果原来的焦点被删除，将焦点移到根并更新棋盘。
        if node_mapping[self.focus].is_none() {
            self.focus = 0;
            event_writer.write(UpdateBoard {
                new_board: self.board()
            });
        } else {
            self.focus = node_mapping[self.focus].unwrap();
        }
    }

    fn show_context_menu(
        &mut self,
        ui: &mut egui::Ui,
        current: usize,
        response: &egui::Response,
        ew_dv: &mut EventWriter<DeleteVariationEvent>,
    ) {
        egui::Popup::context_menu(response)
            .show(|ui| {
            ui.set_min_width(120.0);
            
            if ui.button("Promote Variation").clicked() {
                if let Some(parent) = self.nodes[current].parent {
                    let mut pos = 0;
                    for (idx, (_, son_id, _)) in self.nodes[parent].sons.iter().enumerate() {
                        if *son_id == current {
                            pos = idx;
                        }
                    }
                    if pos != 0 {
                        self.nodes[parent].sons.swap(0, pos);
                    }
                }
            }
            
            if ui.button("Set as mainline").clicked() {
                let mut cur = current;
                while cur != self.root {
                    if let Some(parent) = self.nodes[cur].parent {
                        let mut pos = 0;
                        for (idx, (_, son_id, _)) in self.nodes[parent].sons.iter().enumerate() {
                            if *son_id == cur {
                                pos = idx;
                            }
                        }
                        if pos != 0 {
                            self.nodes[parent].sons.swap(0, pos);
                        }
                        cur = parent;
                    } else {
                        unreachable!()
                    }
                }
            }
            
            if ui.button("Delete Variation").clicked() {
                ew_dv.write(DeleteVariationEvent {
                    node_to_delete: current,
                });
            }
            
            if ui.button("Copy PGN").clicked() {
                ui.ctx().copy_text(self.pgn(current));
            }
        });
    }

    fn get_text_width(s: &String, f: &Fonts, font_id: &FontId) -> f32 {
        let mut res: f32 = 0.0;
        for c in s.chars() {
            res += f.glyph_width(font_id, c);
        }
        res
    }

    fn show_labels_horizontal(
        &mut self, 
        ui: &mut egui::Ui, 
        prefix: String, 
        labels: Vec<(String, usize)>,
        event_writer: &mut EventWriter<UpdateBoard>,
        ew_dv: &mut EventWriter<DeleteVariationEvent>,
    ) {
        let font_id = egui::FontId::default();
        let mono_font = FontId::monospace(14.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new(prefix.clone()).font(mono_font.clone()));
            for (_, (s, idx)) in labels.iter().enumerate() {
                let new_width = ui.fonts(|f| Self::get_text_width(s, f, &font_id));
                let response = ui.allocate_response(
                    egui::Vec2::new(
                        new_width,
                        ui.text_style_height(&egui::TextStyle::Body),
                    ),
                    egui::Sense::click(), 
                );
                if *idx == self.focus {
                    let rect = response.rect;
                    // 使用默认悬停颜色
                    ui.painter().rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
                }
                if response.hovered() {
                    let rect = response.rect;
                    // 使用默认悬停颜色
                    ui.painter().rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
                }
                ui.painter().text(
                    response.rect.left_center(),
                    egui::Align2::LEFT_CENTER,
                    s,
                    font_id.clone(),
                    ui.visuals().text_color(),
                );
                if response.clicked() {
                    self.move_to_node(*idx);
                    event_writer.write(UpdateBoard {
                        new_board: self.board()
                    });
                }
                self.show_context_menu(ui, *idx, &response, ew_dv);
            }
        });
    }

    // 展示一些步，自动换行，换行前使用 header_prefix 作为前缀，换行后使用 prefix 作为前缀
    fn show_labels(
        &mut self, 
        ui: &mut egui::Ui, 
        header_prefix: String, 
        prefix: String, 
        labels: Vec<(String, usize)>,
        event_writer: &mut EventWriter<UpdateBoard>,
        ew_dv: &mut EventWriter<DeleteVariationEvent>,
    ) {
        let font_id = egui::FontId::default();
        let mono_font = FontId::monospace(14.0);
        let initial_width = ui.available_width();
        let mut width = initial_width;
        width -= ui.fonts(|f| Self::get_text_width(&header_prefix, f, &mono_font));
        let mut last = 0;
        let mut first_line = true;
        for (idx, (s, _)) in labels.iter().enumerate() {
            let new_width = ui.fonts(|f| Self::get_text_width(s, f, &font_id));
            if width < new_width + ui.spacing().item_spacing.x {
                self.show_labels_horizontal(
                    ui, 
                    if first_line { header_prefix.clone() } else { prefix.clone() }, 
                    Vec::from(&labels[last..idx]), 
                    event_writer,
                    ew_dv,
                );
                last = idx;
                first_line = false;
                width = initial_width;
                width -= ui.fonts(|f| Self::get_text_width(&prefix, f, &mono_font));
            }
            width -= new_width + ui.spacing().item_spacing.x;
        }
        self.show_labels_horizontal(
            ui, 
            if first_line { header_prefix.clone() } else { prefix.clone() }, 
            Vec::from(&labels[last..]), 
            event_writer,
            ew_dv,
        );
    }

    fn dfs_branch(
        &mut self, 
        current: usize, 
        mut labels: Vec<(String, usize)>,
        header_prefix: String,
        prefix: String,
        ui: &mut egui::Ui,
        event_writer: &mut EventWriter<UpdateBoard>,
        ew_dv: &mut EventWriter<DeleteVariationEvent>,
    ) {
        let son_num = self.nodes[current].sons.len();
        if son_num > 1 {
            self.show_labels(ui, header_prefix.clone(), prefix.clone(), labels, event_writer, ew_dv);

            let new_header = format!("{prefix}{PRE1}");
            let new_pre = format!("{prefix}{PRE3}");
            let last_header = format!("{prefix}{PRE2}");
            let last_pre = format!("{prefix}{PRE4}");

            for i in 0..son_num {
                let (_step, son, move_data) = &self.nodes[current].sons[i];
                let son = *son;
                let san = match move_data.color {
                    PieceColor::White => format!("{}.{}", move_data.ply, move_data.san),
                    PieceColor::Black => format!("{}...{}", move_data.ply, move_data.san),
                };
                
                self.dfs_branch(
                    son,
                    vec![(san, son)],
                    if i == son_num - 1 { last_header.clone() } else { new_header.clone() },
                    if i == son_num - 1 { last_pre.clone() } else { new_pre.clone() },
                    ui,
                    event_writer,
                    ew_dv,
                );
            }
        } else if son_num == 1 {
            let (_step, son, move_data) = &self.nodes[current].sons[0];
            let son = *son;
            let san = match move_data.color {
                PieceColor::White => format!("{}.{}", move_data.ply, move_data.san),
                PieceColor::Black => move_data.san.clone(),
            };
            self.dfs_branch(
                son, 
                {
                    labels.push((san, son));
                    labels
                },
                header_prefix, 
                prefix, 
                ui, 
                event_writer,
                ew_dv);
        } else {
            self.show_labels(ui, header_prefix, prefix, labels, event_writer, ew_dv);
        }
    }

    fn dfs_mainline(
        &mut self, 
        current: usize,
        ui: &mut egui::Ui,
        event_writer: &mut EventWriter<UpdateBoard>,
        ew_dv: &mut EventWriter<DeleteVariationEvent>,
    ) {
        let son_num = self.nodes[current].sons.len();
        let total_width = ui.available_width();

        if son_num >= 1 {
            // 有支线时先展示支线
            for i in 1..son_num {
                let (_step, son, move_data) = &self.nodes[current].sons[i];
                let son = *son;
                let san = match move_data.color {
                    PieceColor::White => format!("{}.{}", move_data.ply, move_data.san),
                    PieceColor::Black => format!("{}...{}", move_data.ply, move_data.san),
                };
                
                self.dfs_branch(
                    son,
                    vec![(san, son)],
                    if i == son_num - 1 { String::from(PRE2) } else { String::from(PRE1) },
                    if i == son_num - 1 { String::from(PRE4) } else { String::from(PRE3) },
                    ui,
                    event_writer,
                    ew_dv,
                );
            }
            let (_step, son, move_data) = self.nodes[current].sons[0].clone();
            match move_data.color {
                PieceColor::White => {
                    ui.horizontal(|ui| {
                        ui.add_sized([total_width * 0.15, 0.0], Label::new(move_data.ply.to_string()));
                        let response = ui.add_sized(
                            [total_width * 0.40, 0.0],
                            Label::new(move_data.san.clone()).sense(Sense::click()),
                        );
                        // if response.secondary_clicked() {
                        //     self.context_menu = Some(son);
                        // }
                        if son == self.focus {
                            let rect = response.rect;
                            // 使用默认悬停颜色
                            ui.painter().rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
                        }
                        if response.hovered() {
                            let rect = response.rect;
                            // 使用默认悬停颜色
                            ui.painter().rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
                        }
                        // 如果文字被颜色覆盖，重新绘制文字
                        if son == self.focus || response.hovered() {
                            let rect = response.rect;
                            ui.painter().text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                move_data.san.clone(),
                                egui::FontId::default(),
                                ui.visuals().text_color(),
                            );
                        }
                        if response.clicked() {
                            self.move_to_node(son);
                            event_writer.write(UpdateBoard {
                                new_board: self.board()
                            });
                        }
                        self.show_context_menu(ui, son, &response, ew_dv);
                        ui.add_sized([total_width * 0.40, 0.0], Label::new("..."));
                    });
                },
                PieceColor::Black => {
                    ui.horizontal(|ui| {
                        ui.add_sized([total_width * 0.15, 0.0], Label::new(move_data.ply.to_string()));
                        ui.add_sized([total_width * 0.40, 0.0], Label::new("..."));
                        let response = ui.add_sized(
                            [total_width * 0.40, 0.0],
                            Label::new(move_data.san.clone()).sense(Sense::click()),
                        );
                        // if response.secondary_clicked() {
                        //     self.context_menu = Some(son);
                        // }
                        if son == self.focus {
                            let rect = response.rect;
                            // 使用默认悬停颜色
                            ui.painter().rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
                        }
                        if response.hovered() {
                            let rect = response.rect;
                            // 使用默认悬停颜色
                            ui.painter().rect_filled(rect, 2.0, ui.visuals().widgets.hovered.bg_fill);
                        }
                        // 如果文字被颜色覆盖，重新绘制文字
                        if son == self.focus || response.hovered() {
                            let rect = response.rect;
                            ui.painter().text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                move_data.san.clone(),
                                egui::FontId::default(),
                                ui.visuals().text_color(),
                            );
                        }
                        if response.clicked() {
                            self.move_to_node(son);
                            event_writer.write(UpdateBoard {
                                new_board: self.board()
                            });
                        }
                        self.show_context_menu(ui, son, &response, ew_dv);
                    });
                },
            }
            self.dfs_mainline(son, ui, event_writer, ew_dv);
        }
    }

    pub fn display_egui(
        &mut self, 
        ui: &mut egui::Ui,
        event_writer: &mut EventWriter<UpdateBoard>,
        ew_dv: &mut EventWriter<DeleteVariationEvent>,
    ) {
        self.dfs_mainline(self.root, ui, event_writer, ew_dv);
    }
}

const PRE1: &str = "├─";
const PRE2: &str = "└─";
const PRE3: &str = "| ";
const PRE4: &str = "  ";
