# CNB 访问资料需求清单 → 已归集，请读正本

> ⚠️ **本文件已改为指针，正文已移走。**
> 原因：与 `CNB-GUIDE.md` 同理，跨项目共用资料按「真理源自一处」统一归集。
>
> **正本 → `F:\fzz-Project\9-CNB\docs\CNB-ACCESS-REQUIREMENTS.md`**
>
> 归集日期：2026-08-31。

---

## 去哪儿看什么

| 你想要的 | 去这里 |
|---|---|
| 所需资料清单 / 权限 scope 对照表 / 安全约定 | `F:\fzz-Project\9-CNB\docs\CNB-ACCESS-REQUIREMENTS.md`（正本） |
| CNB 任务决策树 / 技能索引 | `F:\fzz-Project\9-CNB\docs\CNB-GUIDE.md` |
| 发版全流程 + 13 条踩坑表 | `F:\fzz-Project\9-CNB\docs\CNB发版分发-全流程与踩坑经验.md` |

## sensend 项目专属（正本里没有的）

| 项 | 值 |
|---|---|
| 组织 slug | `fzz198479` |
| 本项目仓库 | `fzz198479/sensend`（**私有**） |
| 本机令牌明文位置 | `F:\fzz-Project\2-sensend\.workbuddy\secrets\CNB-TOKEN.md`（仓外、非 git，防推送泄露） |
| 另一处（简乐本机，solo 用） | `C:\Users\fzz198479.NOVA\.cnb\personal-token` |
| 令牌管理页 | https://cnb.cool/profile/token |

> ⚠️ 令牌**绝不**写进 git 仓内任何文件（docs/、配置、脚本），发版会推到 GitHub/CNB = 泄露。
