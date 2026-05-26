// 注入到 aoe4world overlay 页：透明化 + 加宽 + 徽章修复 + 地图中文 + 对手文明。
// __占位符__ 由 Rust 在创建窗口时替换。
window.addEventListener('DOMContentLoaded', function () {
  var d = document;
  d.documentElement.style.background = 'transparent';
  d.body.style.background = 'transparent';
  d.body.style.opacity = '__OPACITY__';
  d.body.style.zoom = '__ZOOM__';
  d.body.style.display = 'flex';
  d.body.style.justifyContent = '__ALIGN__';
  d.body.style.overflow = 'hidden';
  d.documentElement.style.overflow = 'hidden';

  var fixStyle = d.createElement('style');
  fixStyle.textContent =
    'p.text-sm.uppercase{margin-top:12px !important;}' +
    '[class*="w-[800px]"]{width:1000px !important;}';
  d.head.appendChild(fixStyle);

  // 地图中文：英文名 / 中文（锦标赛前缀剥掉再查表，英文保留）
  var MAP_CN = __MAPCN__;
  var observer = new MutationObserver(function () {
    d.querySelectorAll('p.text-sm.font-bold:not(.uppercase)').forEach(function (el) {
      var text = el.textContent.trim();
      var en = text.indexOf(' / ') >= 0 ? text.split(' / ')[0] : text;
      var key = en.replace(/^[A-Za-z]+(:[A-Za-z]+)? - /, '').replace(/\s*\(old\)\s*$/i, '');
      var cn = MAP_CN[en] || MAP_CN[key];
      if (cn) {
        var expected = en + ' / ' + cn;
        if (text !== expected) el.textContent = expected;
      }
    });
  });
  observer.observe(d.body, { childList: true, subtree: true, characterData: true });

  // 对手常用文明：名字后追加中文文明
  var CIV_CN = __CIVCN__;
  var profileId = '__PROFILE__';
  var lastGameId = null;
  function fetchAndShowCivs() {
    fetch('https://aoe4world.com/api/v0/players/' + profileId + '/games/last')
      .then(function (r) { return r.ok ? r.json() : null; })
      .then(function (game) {
        if (!game || !game.teams || game.game_id === lastGameId) return;
        lastGameId = game.game_id;
        var myTeam = game.teams.find(function (t) {
          return t.some(function (p) { return game.filters.profile_ids.includes(p.profile_id); });
        }) || [];
        var myIds = myTeam.map(function (p) { return p.profile_id; });
        var opponents = game.teams.flat().filter(function (p) { return myIds.indexOf(p.profile_id) < 0; });
        opponents.forEach(function (opp) {
          fetch('https://aoe4world.com/api/v0/players/' + opp.profile_id)
            .then(function (r) { return r.ok ? r.json() : null; })
            .then(function (pData) {
              if (!pData) return;
              var mode = game.leaderboard || 'rm_solo';
              var civs = pData.modes && pData.modes[mode] && pData.modes[mode].civilizations;
              if (!civs || !civs.length) return;
              var top = civs.slice().sort(function (a, b) {
                return (b.pick_rate || 0) - (a.pick_rate || 0);
              }).slice(0, 2);
              var civText = top.map(function (c) { return CIV_CN[c.civilization] || c.civilization; }).join('·');
              d.querySelectorAll('h1.font-bold.text-md.truncate').forEach(function (el) {
                var name = el.textContent.trim();
                if ((name === opp.name || name.indexOf(opp.name + ' (') === 0) && name.indexOf('(') < 0) {
                  el.textContent = opp.name + ' (' + civText + ')';
                }
              });
            })
            .catch(function () {});
        });
      })
      .catch(function () {});
  }
  setTimeout(fetchAndShowCivs, 3000);
  setInterval(fetchAndShowCivs, 20000);
});
