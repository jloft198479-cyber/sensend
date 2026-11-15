# CNB 操作索引指南 → 已归集，请读正本

> ⚠️ **本文件已改为指针，正文已移走。**
> 原因：CNB 资料是跨项目（sensend / solo / book / 网址导航…）共用的，按「真理源自一处」原则统一归集，
> 避免多份副本各自演化后不同步。
>
> **正本 → `F:\fzz-Project\9-CNB\docs\CNB-GUIDE.md`**
>
> 归集日期：2026-08-31。正本版本：2026-08-31（含 Mode A/B 双模式、批量上传脚本、坑位 8/9）。

---

## 去哪儿看什么

| 你想要的 | 去这里 |
|---|---|
| CNB 任务决策树 / 技能索引 / 命令速查 / 环境坑位 | `F:\fzz-Project\9-CNB\docs\CNB-GUIDE.md` |
| 令牌权限 scope 对照 / 怎么申请令牌 | `F:\fzz-Project\9-CNB\docs\CNB-ACCESS-REQUIREMENTS.md` |
| 发版全流程原理（大白话）+ 13 条踩坑表 | `F:\fzz-Project\9-CNB\docs\CNB发版分发-全流程与踩坑经验.md` |
| 可执行技能（发版时加载它） | `F:\fzz-Project\9-CNB\skills\cnb-publish\SKILL.md` |
| 批量上传脚本 | `F:\fzz-Project\9-CNB\skills\cnb-publish\scripts\upload-assets.mjs` |
| 整个 CNB 资料库的入口导航 | `F:\fzz-Project\9-CNB\README.md` |

## sensend 项目专属（正本里没有的）

| 项 | 值 |
|---|---|
| CNB 仓库 | `fzz198479/sensend`（**私有**） |
| git remote 名 | `cnb` → `https://cnb.cool/fzz198479/sensend.git` |
| 用途 | Mode A 国内**代码镜像** + Release 安装包分发 |
| 本机令牌明文 | `F:\fzz-Project\2-sensend\.workbuddy\secrets\CNB-TOKEN.md`（仓外，非 git） |
| 推送必带参数 | `-c http.proxy= -c https.proxy= -c credential.helper=`（本机 git 全局代理连不上 cnb.cool；credential helper 会卡死），并用令牌直连 URL 推送 |

> sensend 的完整发版细节另见本项目 `.workbuddy/memory/MEMORY.md` §「发布到 CNB / cnb.cool 镜像」。
