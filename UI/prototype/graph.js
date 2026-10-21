/* VocaMesh prototype · 关系图控制器
   支持：拖动空白平移、滚轮/按钮缩放、拖动节点、点选节点、节点内“+ / ⋯”操作。 */

(function () {
  const NODE_W = 196;
  const NODE_H = 54;
  const COL = 236;   // 节点宽 + 列间距
  const ROW = 132;   // 层级间距

  function depthOf(team, name) {
    const by = {};
    team.members.forEach(m => { by[m.name] = m; });
    let cur = by[name];
    let d = 0;
    let guard = 0;
    while (cur && cur.name !== team.supervisor && cur.reports && cur.reports !== "用户" && guard++ < 40) {
      const p = by[cur.reports];
      if (!p) break;
      cur = p;
      d++;
    }
    return by[name] && by[name].name === team.supervisor ? 0 : d;
  }

  function layout(team, st, force) {
    const groups = {};
    team.members.forEach(m => {
      if (!force && st.graphPos[m.name]) return;
      const d = depthOf(team, m.name);
      (groups[d] = groups[d] || []).push(m.name);
    });
    const counts = Object.keys(groups).map(k => groups[k].length);
    const maxCount = Math.max.apply(null, counts.concat([1]));
    Object.keys(groups).forEach(k => {
      const names = groups[k];
      const offset = (maxCount - names.length) * COL / 2;
      names.forEach((n, i) => {
        st.graphPos[n] = { x: 40 + offset + i * COL, y: 40 + Number(k) * ROW };
      });
    });
    team.members.forEach(m => {
      if (!st.graphPos[m.name]) st.graphPos[m.name] = { x: 40, y: 40 };
    });
    // 清理已移除成员的坐标
    Object.keys(st.graphPos).forEach(n => {
      if (!team.members.some(m => m.name === n)) delete st.graphPos[n];
    });
  }

  function bounds(team, st) {
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    team.members.forEach(m => {
      const p = st.graphPos[m.name] || { x: 0, y: 0 };
      minX = Math.min(minX, p.x);
      minY = Math.min(minY, p.y);
      maxX = Math.max(maxX, p.x + NODE_W);
      maxY = Math.max(maxY, p.y + NODE_H);
    });
    if (!isFinite(minX)) return { minX: 0, minY: 0, maxX: NODE_W, maxY: NODE_H };
    return { minX, minY, maxX, maxY };
  }

  function applyTransform(plane, st) {
    if (!plane) return;
    plane.style.transform = "translate(" + st.graphPan.x + "px, " + st.graphPan.y + "px) scale(" + st.graphZoom + ")";
  }

  const G = {
    positions(team, st) {
      layout(team, st, false);
      return st.graphPos;
    },

    mount(st) {
      const vp = document.getElementById("graph-viewport");
      const plane = document.getElementById("graph-plane");
      if (!vp || !plane || vp.dataset.bound === "1") return;
      vp.dataset.bound = "1";
      applyTransform(plane, st);

      const nodes = () => Array.from(plane.querySelectorAll(".gnode"));
      const paths = () => Array.from(plane.querySelectorAll(".graph-edges path"));

      function planePoint(clientX, clientY) {
        const r = vp.getBoundingClientRect();
        return { x: (clientX - r.left - st.graphPan.x) / st.graphZoom, y: (clientY - r.top - st.graphPan.y) / st.graphZoom };
      }

      function refreshEdges() {
        const team = (window.STATE.teams || []).find(t => t.id === window.STATE.teamId) || window.STATE.teams[0];
        const pos = {};
        nodes().forEach(n => {
          pos[n.dataset.gnode] = { x: parseFloat(n.style.left) || 0, y: parseFloat(n.style.top) || 0 };
        });
        const items = Array.from(plane.querySelectorAll(".graph-edges path"));
        items.forEach(p => {
          const child = p.dataset.child;
          const parent = p.dataset.parent;
          if (!child || !parent || !pos[child] || !pos[parent]) return;
          const x1 = pos[parent].x + NODE_W / 2, y1 = pos[parent].y + NODE_H;
          const x2 = pos[child].x + NODE_W / 2, y2 = pos[child].y;
          const mid = Math.max(22, (y2 - y1) / 2);
          p.setAttribute("d", "M" + x1 + " " + y1 + " C" + x1 + " " + (y1 + mid) + " " + x2 + " " + (y2 - mid) + " " + x2 + " " + y2);
        });
      }

      let drag = null;

      vp.addEventListener("pointerdown", (e) => {
        if (e.button !== 0) return;
        if (e.target.closest(".gnode-actions")) return;
        const node = e.target.closest(".gnode");
        drag = {
          mode: node ? "node" : "pan",
          name: node ? node.dataset.gnode : null,
          node,
          el: node,
          startX: e.clientX,
          startY: e.clientY,
          originX: node ? parseFloat(node.style.left) || 0 : st.graphPan.x,
          originY: node ? parseFloat(node.style.top) || 0 : st.graphPan.y,
          moved: false,
          panStart: { x: st.graphPan.x, y: st.graphPan.y }
        };
        if (drag.mode === "pan") vp.classList.add("is-panning");
        vp.setPointerCapture(e.pointerId);
      });

      vp.addEventListener("pointermove", (e) => {
        if (!drag) return;
        const dx = e.clientX - drag.startX;
        const dy = e.clientY - drag.startY;
        if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true;
        if (drag.mode === "pan") {
          st.graphPan = { x: drag.panStart.x + dx, y: drag.panStart.y + dy };
          applyTransform(plane, st);
        } else if (drag.el) {
          drag.el.style.left = (drag.originX + dx / st.graphZoom) + "px";
          drag.el.style.top = (drag.originY + dy / st.graphZoom) + "px";
          refreshEdges();
        }
      });

      function endDrag(e) {
        if (!drag) return;
        const d = drag;
        drag = null;
        vp.classList.remove("is-panning");
        try { vp.releasePointerCapture(e.pointerId); } catch (err) { /* noop */ }
        if (d.mode === "node" && d.el) {
          if (!d.moved) {
            window.APP.patch(s => { s.graphSel = d.name; });
            return;
          }
          const p = { x: parseFloat(d.el.style.left) || 0, y: parseFloat(d.el.style.top) || 0 };
          window.APP.patch(s => {
            s.graphPos[d.name] = p;
            s.graphSel = d.name;
          });
        } else if (d.mode === "pan" && d.moved) {
          window.APP.render();
        }
      }
      vp.addEventListener("pointerup", endDrag);
      vp.addEventListener("pointercancel", endDrag);

      vp.addEventListener("wheel", (e) => {
        e.preventDefault();
        const before = planePoint(e.clientX, e.clientY);
        const factor = e.deltaY > 0 ? 0.9 : 1.1;
        const next = Math.min(1.6, Math.max(0.45, st.graphZoom * factor));
        if (next === st.graphZoom) return;
        const r = vp.getBoundingClientRect();
        st.graphZoom = next;
        st.graphPan = {
          x: e.clientX - r.left - before.x * next,
          y: e.clientY - r.top - before.y * next
        };
        applyTransform(plane, st);
        const label = document.querySelector(".graph-zoom");
        if (label) label.textContent = Math.round(next * 100) + "%";
      }, { passive: false });

      vp.addEventListener("click", (e) => {
        const btn = e.target.closest("[data-graph-action]");
        if (btn) {
          const name = btn.dataset.name;
          if (btn.dataset.graphAction === "add-child") window.APP.action("graph-add-child", { dataset: { name } });
          if (btn.dataset.graphAction === "node-menu") window.APP.action("graph-node-menu", btn);
          return;
        }
        const nodeEl = e.target.closest(".gnode");
        if (nodeEl) {
          if (window.STATE.graphSel !== nodeEl.dataset.gnode) window.APP.patch(s => { s.graphSel = nodeEl.dataset.gnode; });
          return;
        }
        if (window.STATE.graphSel) window.APP.patch(s => { s.graphSel = null; });
      });

      vp.addEventListener("keydown", (e) => {
        const node = e.target.closest(".gnode");
        if (!node) return;
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          window.APP.patch(s => { s.graphSel = node.dataset.gnode; });
        }
      });
    },

    zoomBy(st, delta) {
      const next = Math.min(1.6, Math.max(0.45, Math.round((st.graphZoom + delta) * 100) / 100));
      const vp = document.getElementById("graph-viewport");
      if (vp) {
        const r = vp.getBoundingClientRect();
        const cx = r.width / 2, cy = r.height / 2;
        const before = { x: (cx - st.graphPan.x) / st.graphZoom, y: (cy - st.graphPan.y) / st.graphZoom };
        st.graphZoom = next;
        st.graphPan = { x: cx - before.x * next, y: cy - before.y * next };
      } else {
        st.graphZoom = next;
      }
      window.APP.render();
    },

    fit(st, team) {
      const vp = document.getElementById("graph-viewport");
      if (!vp) return;
      const b = bounds(team, st);
      const w = b.maxX - b.minX, h = b.maxY - b.minY;
      const r = vp.getBoundingClientRect();
      const zoom = Math.min(1.4, Math.max(0.45, Math.min((r.width - 80) / w, (r.height - 80) / h)));
      st.graphZoom = Math.round(zoom * 100) / 100;
      st.graphPan = {
        x: (r.width - w * st.graphZoom) / 2 - b.minX * st.graphZoom,
        y: (r.height - h * st.graphZoom) / 2 - b.minY * st.graphZoom
      };
      window.APP.render();
      window.APP.toast("已适应视图（" + Math.round(st.graphZoom * 100) + "%）", "neutral");
    },

    tidy(st, team) {
      layout(team, st, true);
      window.APP.toast("已按层级自动整理节点", "success");
      window.APP.render();
      setTimeout(() => G.fit(window.STATE, team), 30);
    },

    resetPositions(st) {
      st.graphPos = {};
      st.graphPan = { x: 0, y: 0 };
      st.graphZoom = 1;
      window.APP.render();
    }
  };

  window.GRAPH = G;
})();
