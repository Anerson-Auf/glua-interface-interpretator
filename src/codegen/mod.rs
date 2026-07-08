use std::collections::HashSet;
use std::fmt::Write;

use crate::model::{
    BgImageMode, DockMode, ElementKind, GradientDirection, NumAxis, Project, StrExpr, UiElement,
};
use uuid::Uuid;

pub fn generate_glua(project: &Project) -> String {
    let mut out = String::new();
    out.push_str("-- GLua UI (сгенерировано lua_interface_interpretator)\n");
    out.push_str(&format!("-- Проект: {}\n\n", project.name));

    out.push_str("local function CreateUI()\n");
    out.push_str("\tlocal scrW, scrH = ScrW(), ScrH()\n\n");

    let root = project.element(project.root_id).expect("root must exist");

    let ordered = topological_order(project, project.root_id);
    let var_names = unique_var_names(project, &ordered);

    for (id, var) in &var_names {
        let el = project.element(*id).unwrap();
        if *id == project.root_id {
            writeln!(out, "\tlocal {var} = vgui.Create(\"{}\")", el.kind.vgui_class()).unwrap();
        } else {
            let parent_var = parent_var_name(project, el, &var_names);
            writeln!(
                out,
                "\tlocal {var} = vgui.Create(\"{}\", {})",
                el.kind.vgui_class(),
                parent_var
            )
            .unwrap();
        }
        emit_element_setup(&mut out, el, var, project, &var_names);
        out.push('\n');
    }

    out.push_str("\treturn ");
    out.push_str(&sanitize_var(&root.name));
    out.push_str("\nend\n\n");
    out.push_str("return CreateUI\n");

    out
}

fn parent_var_name(project: &Project, el: &UiElement, names: &[(Uuid, String)]) -> String {
    let parent_id = el.parent.unwrap_or(project.root_id);
    names
        .iter()
        .find(|(id, _)| *id == parent_id)
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| "nil".into())
}

fn emit_element_setup(
    out: &mut String,
    el: &UiElement,
    var: &str,
    project: &Project,
    names: &[(Uuid, String)],
) {
    if !el.visible {
        writeln!(out, "\t{var}:SetVisible(false)").unwrap();
    }

    match el.dock {
        DockMode::Fill => writeln!(out, "\t{var}:Dock(FILL)").unwrap(),
        DockMode::Top => writeln!(out, "\t{var}:Dock(TOP)").unwrap(),
        DockMode::Bottom => writeln!(out, "\t{var}:Dock(BOTTOM)").unwrap(),
        DockMode::Left => writeln!(out, "\t{var}:Dock(LEFT)").unwrap(),
        DockMode::Right => writeln!(out, "\t{var}:Dock(RIGHT)").unwrap(),
        DockMode::None => {
            let (xg, yg, wg, hg) = if el.parent.is_some() {
                let pv = parent_var_name(project, el, names);
                (
                    el.x.to_glua_for_child(&pv, NumAxis::X),
                    el.y.to_glua_for_child(&pv, NumAxis::Y),
                    el.w.to_glua_for_child(&pv, NumAxis::W),
                    el.h.to_glua_for_child(&pv, NumAxis::H),
                )
            } else {
                (
                    el.x.to_glua(),
                    el.y.to_glua(),
                    el.w.to_glua(),
                    el.h.to_glua(),
                )
            };
            writeln!(out, "\t{var}:SetPos({xg}, {yg})").unwrap();
            writeln!(out, "\t{var}:SetSize({wg}, {hg})").unwrap();
        }
    }

    emit_clear_builtin_text(out, el, var);

    match el.kind {
        ElementKind::Image => {
            let mat = material_expr(el);
            writeln!(out, "\t{var}:SetMaterial({mat})").unwrap();
        }
        _ => {}
    }

    if el.kind == ElementKind::TextEntry {
        writeln!(
            out,
            "\t{var}:SetDrawBackground({})",
            bool_lua(el.text_entry_draw_background)
        )
        .unwrap();
        writeln!(
            out,
            "\t{var}:SetDrawBorder({})",
            bool_lua(el.text_entry_draw_border)
        )
        .unwrap();
        writeln!(
            out,
            "\t{var}:SetMultiline({})",
            bool_lua(el.text_entry_multiline)
        )
        .unwrap();
        writeln!(
            out,
            "\t{var}:SetNumeric({})",
            bool_lua(el.text_entry_numeric)
        )
        .unwrap();
        writeln!(
            out,
            "\t{var}:SetEditable({})",
            bool_lua(el.text_entry_editable)
        )
        .unwrap();
    }

    if el.make_popup {
        writeln!(out, "\t{var}:MakePopup()").unwrap();
    }

    if el.close_on_escape {
        writeln!(out, "\t{var}.OnKeyCodeReleased = function(self, key)").unwrap();
        writeln!(out, "\t\tif key == KEY_ESCAPE then self:Remove() end").unwrap();
        writeln!(out, "\tend").unwrap();
    }

    if el.kind == ElementKind::Button && el.button_disabled {
        writeln!(out, "\t{var}:SetEnabled(false)").unwrap();
    }

    emit_cached_ui_colors(out, el, var);

    if el.uses_custom_paint() {
        emit_combined_paint(out, el, var);
    } else if el.paint_background && el.bg_color[3] > 0 {
        emit_solid_paint(out, el, var);
    }

    emit_do_click(out, el, var);
    emit_on_hover(out, el, var);

    if el.close_button.enabled {
        emit_generated_close_button(out, el, var, project, names);
    }
}

fn emit_do_click(out: &mut String, el: &UiElement, var: &str) {
    let code = el.lua_do_click.trim();
    if code.is_empty() {
        return;
    }
    writeln!(out, "\t{var}.DoClick = function(self)").unwrap();
    for line in code.lines() {
        writeln!(out, "\t\t{line}").unwrap();
    }
    writeln!(out, "\tend").unwrap();
}

fn emit_on_hover(out: &mut String, el: &UiElement, var: &str) {
    let code = el.lua_on_hover.trim();
    if code.is_empty() {
        return;
    }
    writeln!(out, "\t{var}.Think = function(self)").unwrap();
    writeln!(out, "\t\tif self:IsHovered() then").unwrap();
    for line in code.lines() {
        writeln!(out, "\t\t\t{line}").unwrap();
    }
    writeln!(out, "\t\tend").unwrap();
    writeln!(out, "\tend").unwrap();
}

fn emit_generated_close_button(
    out: &mut String,
    el: &UiElement,
    var: &str,
    project: &Project,
    names: &[(Uuid, String)],
) {
    let cb = &el.close_button;
    let cb_var = format!("{var}_CloseBtn");
    writeln!(
        out,
        "\tlocal {cb_var} = vgui.Create(\"DButton\", {var})"
    )
    .unwrap();
    let xg = cb.x.to_glua_for_child(var, NumAxis::X);
    let yg = cb.y.to_glua_for_child(var, NumAxis::Y);
    writeln!(out, "\t{cb_var}:SetPos({xg}, {yg})").unwrap();
    writeln!(
        out,
        "\t{cb_var}:SetSize({:.0}, {:.0})",
        cb.w, cb.h
    )
    .unwrap();
    writeln!(
        out,
        "\t{cb_var}:SetText(\"{}\")",
        escape_lua(&cb.label)
    )
    .unwrap();
    let click = if cb.lua_do_click.trim().is_empty() {
        "self:GetParent():Remove()"
    } else {
        cb.lua_do_click.trim()
    };
    writeln!(out, "\t{cb_var}.DoClick = function(self)").unwrap();
    for line in click.lines() {
        writeln!(out, "\t\t{line}").unwrap();
    }
    writeln!(out, "\tend").unwrap();
    let _ = (project, names);
}

fn emit_clear_builtin_text(out: &mut String, el: &UiElement, var: &str) {
    match el.kind {
        ElementKind::TextEntry => {
            writeln!(out, "\t{var}:SetValue(\"\")").unwrap();
        }
        ElementKind::Button => {
            writeln!(out, "\t{var}:SetText(\"\")").unwrap();
        }
        ElementKind::Frame => {
            writeln!(out, "\t{var}:SetTitle(\"\")").unwrap();
        }
        _ => {}
    }
}

fn color_glua(c: [u8; 4]) -> String {
    format!("Color({}, {}, {}, {})", c[0], c[1], c[2], c[3])
}

fn emit_self_color(out: &mut String, var: &str, field: &str, c: [u8; 4]) {
    writeln!(out, "\t{var}.{field} = {}", color_glua(c)).unwrap();
}

/// Цвета создаются один раз при setup, в Paint только ссылки на self._UI*.
fn emit_cached_ui_colors(out: &mut String, el: &UiElement, var: &str) {
    let needs_bg = el.paint_background && el.bg_color[3] > 0;
    if needs_bg {
        emit_self_color(out, var, "_UIBg", el.bg_color);
    }

    if el.kind == ElementKind::Button {
        if el.button_disabled {
            let c = el.bg_color;
            emit_self_color(
                out,
                var,
                "_UIBgDisabled",
                [c[0] / 2, c[1] / 2, c[2] / 2, c[3]],
            );
        }
        if el.button_hover_enabled {
            emit_self_color(out, var, "_UIBgHover", el.button_hover_bg);
        }
        if el.button_pressed_enabled {
            emit_self_color(out, var, "_UIBgPressed", el.button_pressed_bg);
        }

        if button_text_states_enabled(el) {
            emit_self_color(out, var, "_UITextHover", el.button_hover_text_color);
            emit_self_color(out, var, "_UITextPressed", el.button_pressed_text_color);
            if el.button_disabled {
                let tc = el.text_color;
                emit_self_color(
                    out,
                    var,
                    "_UITextDisabled",
                    [tc[0] / 2, tc[1] / 2, tc[2] / 2, tc[3]],
                );
            }
        }
    }

    if !el.text_layers.is_empty() {
        for (i, layer) in el.text_layers.iter().enumerate() {
            if !is_empty_text(&layer.text) {
                emit_self_color(out, var, &format!("_UIText{i}"), layer.text_color);
            }
        }
    } else if el.kind == ElementKind::Button && !is_empty_text(&el.text) {
        emit_self_color(out, var, "_UIText", el.text_color);
    }

    if el.bg_gradient.enabled {
        emit_self_color(out, var, "_UIGradStart", el.bg_gradient.color_start);
        emit_self_color(out, var, "_UIGradEnd", el.bg_gradient.color_end);
        writeln!(
            out,
            "\t{var}._UIGradSteps = {}",
            el.bg_gradient.steps.clamp(4, 128)
        )
        .unwrap();
    }
}

fn emit_solid_paint(out: &mut String, el: &UiElement, var: &str) {
    if el.kind == ElementKind::TextEntry {
        return;
    }
    writeln!(out, "\t{var}.Paint = function(self, w, h)").unwrap();
    if el.kind == ElementKind::Button
        && (el.button_hover_enabled || el.button_pressed_enabled || el.button_disabled)
    {
        emit_pick_self_bg_color(out, el);
    } else {
        emit_rounded_box_self(out, el.corner_radius, "\t\t");
    }
    if el.kind == ElementKind::Button {
        emit_button_text_in_paint(out, el);
    }
    writeln!(out, "\tend").unwrap();
}

fn emit_rounded_box_self(out: &mut String, radius: f32, indent: &str) {
    writeln!(
        out,
        "{indent}draw.RoundedBox({radius:.0}, 0, 0, w, h, self._UIBg)"
    )
    .unwrap();
}

fn emit_pick_self_bg_color(out: &mut String, el: &UiElement) {
    writeln!(out, "\t\tlocal col = self._UIBg").unwrap();
    emit_self_color_if_chain(
        out,
        "\t\t",
        &[
            (
                el.button_disabled,
                "not self:IsEnabled()",
                "self._UIBgDisabled",
            ),
            (el.button_pressed_enabled, "self:IsDown()", "self._UIBgPressed"),
            (
                el.button_hover_enabled,
                "self:IsHovered() and not self:IsDown()",
                "self._UIBgHover",
            ),
        ],
        "col",
    );
    writeln!(
        out,
        "\t\tdraw.RoundedBox({:.0}, 0, 0, w, h, col)",
        el.corner_radius
    )
    .unwrap();
}

/// Генерирует if / elseif / end для выбора поля self._UI* без Color() в Paint.
fn emit_self_color_if_chain(
    out: &mut String,
    indent: &str,
    branches: &[(bool, &str, &str)],
    target: &str,
) {
    let active: Vec<_> = branches.iter().filter(|(en, _, _)| *en).copied().collect();
    if active.is_empty() {
        return;
    }
    for (i, (_, cond, src)) in active.iter().enumerate() {
        let kw = if i == 0 { "if" } else { "elseif" };
        writeln!(out, "{indent}{kw} {cond} then").unwrap();
        writeln!(out, "{indent}\t{target} = {src}").unwrap();
    }
    writeln!(out, "{indent}end").unwrap();
}

fn emit_pick_self_text_color(out: &mut String, el: &UiElement, base_field: &str, indent: &str) {
    if !button_text_states_enabled(el) {
        return;
    }
    writeln!(out, "{indent}local textCol = self.{base_field}").unwrap();
    emit_self_color_if_chain(
        out,
        indent,
        &[
            (
                el.button_disabled,
                "not self:IsEnabled()",
                "self._UITextDisabled",
            ),
            (el.button_pressed_enabled, "self:IsDown()", "self._UITextPressed"),
            (
                el.button_hover_enabled,
                "self:IsHovered() and not self:IsDown()",
                "self._UITextHover",
            ),
        ],
        "textCol",
    );
}

fn emit_gradient_paint_body(out: &mut String, el: &UiElement) {
    writeln!(out, "\t\tlocal steps = self._UIGradSteps").unwrap();
    writeln!(out, "\t\tlocal gs, ge = self._UIGradStart, self._UIGradEnd").unwrap();
    match el.bg_gradient.direction {
        GradientDirection::Vertical => {
            writeln!(out, "\t\tfor i = 0, steps do").unwrap();
            writeln!(out, "\t\t\tlocal t = i / steps").unwrap();
            writeln!(out, "\t\t\tlocal r = Lerp(t, gs.r, ge.r)").unwrap();
            writeln!(out, "\t\t\tlocal g = Lerp(t, gs.g, ge.g)").unwrap();
            writeln!(out, "\t\t\tlocal b = Lerp(t, gs.b, ge.b)").unwrap();
            writeln!(out, "\t\t\tlocal a = Lerp(t, gs.a, ge.a)").unwrap();
            writeln!(out, "\t\t\tsurface.SetDrawColor(r, g, b, a)").unwrap();
            writeln!(out, "\t\t\tsurface.DrawRect(0, h * t, w, h / steps + 1)").unwrap();
            writeln!(out, "\t\tend").unwrap();
        }
        GradientDirection::Horizontal => {
            writeln!(out, "\t\tfor i = 0, steps do").unwrap();
            writeln!(out, "\t\t\tlocal t = i / steps").unwrap();
            writeln!(out, "\t\t\tlocal r = Lerp(t, gs.r, ge.r)").unwrap();
            writeln!(out, "\t\t\tlocal g = Lerp(t, gs.g, ge.g)").unwrap();
            writeln!(out, "\t\t\tlocal b = Lerp(t, gs.b, ge.b)").unwrap();
            writeln!(out, "\t\t\tlocal a = Lerp(t, gs.a, ge.a)").unwrap();
            writeln!(out, "\t\t\tsurface.SetDrawColor(r, g, b, a)").unwrap();
            writeln!(out, "\t\t\tsurface.DrawRect(w * t, 0, w / steps + 1, h)").unwrap();
            writeln!(out, "\t\tend").unwrap();
        }
    }
}

fn emit_combined_paint(out: &mut String, el: &UiElement, var: &str) {
    let mat_path = el.export_material_path();
    let is_imaged = el.kind == ElementKind::EditablePanelImaged && !mat_path.is_empty();

    if is_imaged {
        writeln!(
            out,
            "\t{var}.BgMat = Material(\"{}\")",
            escape_lua(&mat_path)
        )
        .unwrap();
    }

    writeln!(out, "\t{var}.Paint = function(self, w, h)").unwrap();

    if is_imaged {
        emit_imaged_paint_body(out, el);
    }

    if el.bg_gradient.enabled {
        emit_gradient_paint_body(out, el);
    } else if el.paint_background && el.bg_color[3] > 0 && !is_imaged {
        if el.kind == ElementKind::Button
            && (el.button_hover_enabled || el.button_pressed_enabled || el.button_disabled)
        {
            emit_pick_self_bg_color(out, el);
        } else {
            emit_rounded_box_self(out, el.corner_radius, "\t\t");
        }
    }

    if is_imaged && el.corner_radius > 0.0 && el.bg_color[3] > 0 {
        writeln!(
            out,
            "\t\tdraw.RoundedBox({:.0}, 0, 0, w, h, self._UIBg)",
            el.corner_radius
        )
        .unwrap();
    }

    for layer in &el.image_layers {
        let mat = layer.export_material();
        if mat.is_empty() {
            continue;
        }
        let alpha = layer.alpha;
        writeln!(
            out,
            "\t\tsurface.SetDrawColor(255, 255, 255, {alpha})"
        )
        .unwrap();
        writeln!(
            out,
            "\t\tsurface.SetMaterial(Material(\"{}\"))",
            escape_lua(&mat)
        )
        .unwrap();
        writeln!(
            out,
            "\t\tsurface.DrawTexturedRect({}, {}, {}, {})",
            layer.x.to_glua_in_parent(),
            layer.y.to_glua_in_parent(),
            layer.w.to_glua_in_parent(),
            layer.h.to_glua_in_parent()
        )
        .unwrap();
    }

    for (i, layer) in el.text_layers.iter().enumerate() {
        emit_text_layer_in_paint(out, el, layer, i);
    }

    if el.kind == ElementKind::Button {
        emit_button_builtin_text_in_paint(out, el);
    }

    writeln!(out, "\tend").unwrap();
}

fn button_text_states_enabled(el: &UiElement) -> bool {
    el.kind == ElementKind::Button
        && (el.button_hover_enabled || el.button_pressed_enabled || el.button_disabled)
}

fn emit_button_text_in_paint(out: &mut String, el: &UiElement) {
    for (i, layer) in el.text_layers.iter().enumerate() {
        emit_text_layer_in_paint(out, el, layer, i);
    }
    emit_button_builtin_text_in_paint(out, el);
}

fn emit_text_layer_in_paint(
    out: &mut String,
    el: &UiElement,
    layer: &crate::model::TextLayer,
    layer_index: usize,
) {
    if is_empty_text(&layer.text) {
        return;
    }
    let text = layer.text.to_glua();
    let base_field = format!("_UIText{layer_index}");
    if button_text_states_enabled(el) {
        writeln!(out, "\t\tdo").unwrap();
        emit_pick_self_text_color(out, el, &base_field, "\t\t\t");
        writeln!(
            out,
            "\t\t\tdraw.SimpleText({text}, \"{}\", {}, {}, textCol, {})",
            font_for_draw(&layer.font_name),
            layer.x.to_glua_in_parent(),
            layer.y.to_glua_in_parent(),
            layer.align.glua_const()
        )
        .unwrap();
        writeln!(out, "\t\tend").unwrap();
    } else {
        writeln!(
            out,
            "\t\tdraw.SimpleText({text}, \"{}\", {}, {}, self.{base_field}, {})",
            font_for_draw(&layer.font_name),
            layer.x.to_glua_in_parent(),
            layer.y.to_glua_in_parent(),
            layer.align.glua_const()
        )
        .unwrap();
    }
}

fn emit_button_builtin_text_in_paint(out: &mut String, el: &UiElement) {
    if el.kind != ElementKind::Button || !el.text_layers.is_empty() {
        return;
    }
    if is_empty_text(&el.text) {
        return;
    }
    let text = el.text.to_glua();
    let font = font_for_draw(&el.font_name);
    if button_text_states_enabled(el) {
        writeln!(out, "\t\tdo").unwrap();
        emit_pick_self_text_color(out, el, "_UIText", "\t\t\t");
        writeln!(
            out,
            "\t\t\tdraw.SimpleText({text}, \"{font}\", w / 2, h / 2, textCol, TEXT_ALIGN_CENTER)"
        )
        .unwrap();
        writeln!(out, "\t\tend").unwrap();
    } else {
        writeln!(
            out,
            "\t\tdraw.SimpleText({text}, \"{font}\", w / 2, h / 2, self._UIText, TEXT_ALIGN_CENTER)"
        )
        .unwrap();
    }
}

fn emit_imaged_paint_body(out: &mut String, el: &UiElement) {
    let alpha = el.bg_image_alpha;
    let tile = el.bg_image_tile_size;

    match el.bg_image_mode {
        BgImageMode::Stretch => {
            writeln!(out, "\t\tsurface.SetDrawColor(255, 255, 255, {alpha})").unwrap();
            writeln!(out, "\t\tsurface.SetMaterial(self.BgMat)").unwrap();
            writeln!(out, "\t\tsurface.DrawTexturedRect(0, 0, w, h)").unwrap();
        }
        BgImageMode::Tile => {
            writeln!(out, "\t\tsurface.SetDrawColor(255, 255, 255, {alpha})").unwrap();
            writeln!(out, "\t\tsurface.SetMaterial(self.BgMat)").unwrap();
            writeln!(out, "\t\tlocal tw, th = {tile}, {tile}").unwrap();
            writeln!(out, "\t\tfor x = 0, w, tw do").unwrap();
            writeln!(out, "\t\t\tfor y = 0, h, th do").unwrap();
            writeln!(out, "\t\t\t\tsurface.DrawTexturedRect(x, y, tw, th)").unwrap();
            writeln!(out, "\t\t\tend").unwrap();
            writeln!(out, "\t\tend").unwrap();
        }
        BgImageMode::Cover => {
            writeln!(out, "\t\tsurface.SetDrawColor(255, 255, 255, {alpha})").unwrap();
            writeln!(out, "\t\tsurface.SetMaterial(self.BgMat)").unwrap();
            writeln!(out, "\t\tlocal mw, mh = self.BgMat:Width(), self.BgMat:Height()").unwrap();
            writeln!(out, "\t\tif mw > 0 and mh > 0 then").unwrap();
            writeln!(out, "\t\t\tlocal scale = math.max(w / mw, h / mh)").unwrap();
            writeln!(out, "\t\t\tlocal dw, dh = mw * scale, mh * scale").unwrap();
            writeln!(
                out,
                "\t\t\tsurface.DrawTexturedRect((w - dw) / 2, (h - dh) / 2, dw, dh)"
            )
            .unwrap();
            writeln!(out, "\t\telse").unwrap();
            writeln!(out, "\t\t\tsurface.DrawTexturedRect(0, 0, w, h)").unwrap();
            writeln!(out, "\t\tend").unwrap();
        }
    }
}

fn material_expr(el: &UiElement) -> String {
    let path = el.export_material_path();
    if path.is_empty() {
        "Material(\"icon16/picture.png\")".into()
    } else {
        format!("Material(\"{}\")", escape_lua(&path))
    }
}

fn font_for_draw(name: &str) -> String {
    if name.is_empty() {
        "DermaDefault".into()
    } else {
        escape_lua(name)
    }
}

fn bool_lua(v: bool) -> &'static str {
    if v {
        "true"
    } else {
        "false"
    }
}

fn is_empty_text(text: &StrExpr) -> bool {
    matches!(text, StrExpr::Literal(s) if s.is_empty())
}

fn escape_lua(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn unique_var_names(project: &Project, ordered: &[Uuid]) -> Vec<(Uuid, String)> {
    let mut used = HashSet::new();
    ordered
        .iter()
        .map(|id| {
            let el = project.element(*id).unwrap();
            let base = sanitize_var(&el.name);
            let mut candidate = base.clone();
            let mut suffix = 2;
            while used.contains(&candidate) {
                candidate = format!("{base}_{suffix}");
                suffix += 1;
            }
            used.insert(candidate.clone());
            (el.id, candidate)
        })
        .collect()
}

fn sanitize_var(name: &str) -> String {
    let mut s: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.is_empty() || s.chars().next().is_some_and(|c| c.is_ascii_digit()) || is_lua_keyword(&s) {
        s = format!("p_{s}");
    }
    s
}

fn is_lua_keyword(s: &str) -> bool {
    matches!(
        s,
        "and"
            | "break"
            | "do"
            | "else"
            | "elseif"
            | "end"
            | "false"
            | "for"
            | "function"
            | "goto"
            | "if"
            | "in"
            | "local"
            | "nil"
            | "not"
            | "or"
            | "repeat"
            | "return"
            | "then"
            | "true"
            | "until"
            | "while"
    )
}

fn topological_order(project: &Project, root: Uuid) -> Vec<Uuid> {
    let mut result = Vec::new();
    visit(project, root, &mut result);
    result
}

fn visit(project: &Project, id: Uuid, out: &mut Vec<Uuid>) {
    out.push(id);
    for child_id in project.children_ids(id) {
        visit(project, child_id, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ElementKind, Project, UiElement};

    #[test]
    fn editable_panel_export_does_not_call_missing_set_text() {
        let project = Project::default();

        let code = generate_glua(&project);

        assert!(!code.contains(":SetText(\"\")"));
    }

    #[test]
    fn frame_export_clears_title_with_frame_api() {
        let mut project = Project::default();
        let root_id = project.root_id;
        let frame_id = project.add_element(UiElement::new(ElementKind::Frame, "Frame"), root_id);

        let code = generate_glua(&project);
        let names = unique_var_names(&project, &topological_order(&project, project.root_id));
        let frame_var = names
            .iter()
            .find(|(id, _)| *id == frame_id)
            .map(|(_, name)| name.as_str())
            .unwrap();

        assert!(code.contains(&format!("{frame_var}:SetTitle(\"\")")));
        assert!(!code.contains(&format!("{frame_var}:SetText(\"\")")));
    }

    #[test]
    fn duplicate_and_keyword_element_names_get_safe_lua_locals() {
        let mut project = Project::default();
        let root_id = project.root_id;
        let first = project.add_element(UiElement::new(ElementKind::Panel, "end"), root_id);
        let second = project.add_element(UiElement::new(ElementKind::Panel, "end"), root_id);
        project.add_element(UiElement::new(ElementKind::Panel, "child"), first);
        project.add_element(UiElement::new(ElementKind::Panel, "sibling"), second);

        let code = generate_glua(&project);

        assert!(code.contains("local p_end = vgui.Create(\"DPanel\", RootPanel)"));
        assert!(code.contains("local p_end_2 = vgui.Create(\"DPanel\", RootPanel)"));
        assert!(code.contains("local child = vgui.Create(\"DPanel\", p_end)"));
        assert!(code.contains("local sibling = vgui.Create(\"DPanel\", p_end_2)"));
    }
}
