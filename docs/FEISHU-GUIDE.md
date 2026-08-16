# 飞书对接指南（Sensend）

> 给"明天就忘"的自己：飞书这套机制不需要背，知道往哪查就行。本文按「逻辑 → 配置 → 排查」三段写，重点把最容易晕的**权限模型**讲透。
> 适用场景：sensend 通过「企业自建应用」把笔记追加（Append）到指定的飞书文档 / 知识库页面。

---

## 〇、三句话先记住

1. sensend 用的是「应用身份」（App ID + App Secret），不碰你的飞书账号密码。
2. 能不能写一份文档，取决于 **三个条件同时满足**：API 权限（Scope）✅ + 权限已发布 ✅ + 应用被加为文档/知识库协作者 ✅。
3. **读和写是两套独立权限**，可能出现「能读不能写」——这就是 2026-08-17 排查到的真实问题。

---

## 一、飞书的整套逻辑（大白话版）

### 1.1 身份：到底是谁在访问？

- App ID / App Secret 相当于「公司门禁卡 + 密码」。
- 用它们换来的 `tenant_access_token` 相当于「临时通行证」，**2 小时有效**。
- 通行证代表「应用自己」，不是某个员工。所以 sensend 干的每件事，飞书都理解为「这个应用在操作」。

一次完整流程（对应 [lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs#L472-L498)）：

```
App ID + App Secret
        │  ① 换通行证
        ▼
tenant_access_token（2 小时，进程内缓存）
        │  ② 拿通行证去开门
        ▼
读：解析 wiki 链接 → 找到文档 → 读取文档
写：把笔记转成 block → 追加到文档末尾
```

### 1.2 权限 = 三扇门，全开才能写

这是飞书最容易晕的地方：**光有 token 没用，要一层一层把门打开**。

| 门 | 在哪设置 | 管什么 | 最容易踩的坑 |
|---|---|---|---|
| ① **API 权限 Scope** | 开放平台 → 权限管理 | 这个应用「能不能调用某个接口」（比如能不能写文档） | 加了权限但**没发布版本**，线上不生效 |
| ② **版本发布** | 开放平台 → 版本管理与发布 | 让 ① 的权限真正生效 | 改完权限忘了发版 = 白改 |
| ③ **资源协作** | 飞书客户端，把应用加进文档/知识库协作者 | 这个应用「能不能碰到那份**具体**文档」 | 有权限但没加协作者，连看都看不了 |

打个比方：

- ① = 你**会不会开车**（驾照）
- ② = 驾照有没有**年审**（过期了照扣）
- ③ = 你有没有**这辆车的钥匙**（驾照再全，没钥匙也开不走）

三样缺一不可。

### 1.3 为什么「能读不能写」？（今天踩的坑）

- 读文档只需要权限 A；写文档（追加块）需要权限 A + 写权限 B。
- 2026-08-17 实测：**读正常，写返回 `403 / code=1770032 / forBidden`**，连最普通的 text 块都被拒。
- 说明该应用只有读权限，没有写权限（或写了权限但没发布）。
- **注意**：sensend 配置窗口的「测试连接」按钮只做了**读取**验证（拿 token → 解析 wiki → 读文档，见 [lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs#L455-L470)），**从不测写入**。所以「测试连接显示正常」≠「能写入」。真正的写入验证 = 实际发一条内容。

### 1.4 几个名词别搞混

| 名词 | 是什么 | sensend 里对应 |
|---|---|---|
| App ID | 应用身份证号 | 配置里的 **token** 字段 |
| App Secret | 应用密码 | 配置里的 **token2** 字段 |
| tenant_access_token | 应用身份的临时通行证（2h） | 每次 API 调用都用它 |
| node_token | 知识库页面的「地址编号」 | 从 wiki 链接里取（URL 最后一段） |
| document_id / obj_token | 真正文档的「身份证号」 | 写入文档时用 |
| Block | 文档里一段一段的内容块 | sensend 把笔记转成 block 再追加 |

wiki 链接解析流程（[lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs#L176-L214)）：

```
粘贴的 wiki 链接: https://xxx.feishu.cn/wiki/GydowmeXUiXRWIk3vFicLtHVngd
                          │  提取最后一段
                          ▼
node_token = GydowmeXUiXRWIk3vFicLtHVngd
                          │  调 get_node 解析
                          ▼
obj_token = QNxAdoxSaom98ExNim2cCG0Wn9c  ← 这就是文档的 document_id，用它追加内容
```

### 1.5 为什么 sensend 只能「追加」不能「覆盖/编辑」

- 用应用身份（tenant_access_token）写文档，飞书只允许做「在文档末尾加块」这类温和操作。
- 这是飞书的限制，不是 bug。所以 sensend 的飞书只支持 `page`（追加）模式。

### 1.6 内容长什么样？（block_type 映射，以代码为准）

飞书文档 = 一个根块 + 无数子块。sensend 把富文本笔记转成这些块（常量定义在 [lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs#L13-L21)）：

| block_type | 含义 | sensend 从哪来 |
|---|---|---|
| 2 | 文本 | 段落 |
| 3~11 | 标题（heading1~9） | 标题 |
| 12 | 无序列表 bullet | 项目符号列表 |
| 13 | 有序列表 ordered | 编号列表 |
| 14 | 代码 code | 代码块 |
| 15 | 引用 quote | 引用 |
| 17 | 待办 todo（style.done 控制勾选） | 任务列表 |
| 22 | 分割线 divider | 分隔线 |

> 注：历史文档 CODE-WIKI.md 曾把 code 写成 17，**实际 code=14、todo=17**（以 lark.rs 代码为准）。

---

## 二、如何有效配置（照抄即可）

### 2.1 飞书开放平台侧（一次配置，长期有效）

1. 打开 https://open.feishu.cn → 创建 **企业自建应用**，填个名字（如 "Agent整理笔记"）。
2. 左侧「凭证与基础信息」→ 记下 **App ID** 和 **App Secret**（Secret 只完整显示一次，立即备份）。
3. 左侧「权限管理」→ 搜索并开通以下**租户权限**：
   - `docx:document`（创建及编辑新版文档）— 文档读写，写文档的必须
   - `wiki:wiki`（查看、编辑和管理知识库）— 因为目标是知识库页面，解析链接需要它
4. **关键一步**：左侧「版本管理与发布」→ 创建版本 → 申请发布。
   - 个人自建应用通常免审 / 自动通过；企业应用需管理员审核。
   - **不发布 = 上面开的权限全部无效**。
5. 给应用「分钥匙」（资源协作）：
   - 目标是知识库页面 → 进知识库 → 设置 → 成员与权限 → 添加成员 → 选「机器人」→ 搜你的应用 → 授权 **可编辑**（至少编辑权限，才能写入）。
   - 目标是普通云文档 → 文档右上角「...」→ 更多 → 添加文档应用 → 搜应用 → 授予**可编辑**。

### 2.2 Sensend 侧

1. 打开配置窗口 → 新增/编辑「飞书」实例。
2. `App ID` 填入 **token** 字段，`App Secret` 填入 **token2** 字段（对应 [config.json](file:///c:/Users/fzz198479.NOVA/AppData/Roaming/com.jloft.sensend/config.json) 里 lark 实例的 token / token2）。
3. 目标粘贴 wiki 链接：`https://xxx.feishu.cn/wiki/xxxx`。
4. 点「测试连接」——只验证读权限，成功即可保存。
5. **再真发一条内容**，验证写权限（这一步才算闭环）。

### 2.3 验证权限是否生效（可选）

用 PowerShell 直接调飞书 API 自查，最快：

```powershell
# 1) 换 token
$body = @{ app_id = '<你的App ID>'; app_secret = '<你的App Secret>' } | ConvertTo-Json
$r = Invoke-RestMethod -Uri 'https://open.feishu.cn/open-apis/auth/v3/tenant_access_token/internal' -Method Post -ContentType 'application/json; charset=utf-8' -Body $body
$tok = $r.tenant_access_token
$h = @{ Authorization = "Bearer $tok" }

# 2) 读：解析 wiki 链接（拿 node_token）
$node = Invoke-RestMethod -Uri 'https://open.feishu.cn/open-apis/wiki/v2/spaces/get_node?token=<node_token>' -Headers $h
$doc = $node.data.node.obj_token

# 3) 读：读取文档
Invoke-RestMethod -Uri "https://open.feishu.cn/open-apis/docx/v1/documents/$doc" -Headers $h

# 4) 写：追加一个文本块（验证写权限！）
$payload = @{ children = @( @{ block_type = 2; text = @{ elements = @( @{ text_run = @{ content = "权限验证" } } ); style = @{} } } ) } | ConvertTo-Json -Depth 10
Invoke-RestMethod -Uri "https://open.feishu.cn/open-apis/docx/v1/documents/$doc/blocks/$doc/children" -Method Post -Headers $h -ContentType 'application/json; charset=utf-8' -Body $payload
```

第 4 步返回 `code=0` 才算写入权限 OK；返回 `403` 就是权限问题。

---

## 三、关键问题排查表

| 现象 | 大概率原因 | 处理 |
|---|---|---|
| 提示「无权限访问目标，请检查 Token 权限」（本质是 403） | **写入权限缺失**：没开写权限 / 权限没发布 / 应用只有读权限 / 被移出协作者 | 按排查顺序检查下面「三扇门」 |
| 认证失败（code≠0） | App ID / Secret 填错、抄错、带空格 | 重新复制凭证，比对 config.json |
| 报「wiki 中嵌入的类型不支持」 | 链接指向的不是文档（如电子表格、多维表） | 换成普通文档 / 文档型 wiki 页 |
| 发送「成功」但飞书里看不到 | 追加模式，内容加在文档**末尾** | 刷新页面，往下滚动 |
| 换了 Secret 但行为没变 | 进程内 token 缓存 2 小时未过期 | 重启应用（缓存即清） |
| 报频控错误（99991400 / HTTP 429） | 超频率限制：单应用 3 次/秒、单文档 3 次/秒 | 稍等重试；sensend 单次发送请求很少，正常不会触发 |

### 排查顺序口诀

> **先看权限开没开 → 再看发没发版 → 再看协作者加没加 → 最后才怀疑代码。**

展开讲：
1. 权限管理里有没有 `docx:document`（写文档）和 `wiki:wiki`（解析知识库）？
2. 版本管理与发布里，最近一次发版是否在「改权限之后」？
3. 飞书客户端里，目标文档/知识库的协作者列表里有没有这个应用？权限是「可编辑」还是只有「可查看」？
4. 以上都对还不行，再用 2.3 的脚本直接调 API，看具体错误码和 msg。

---

## 四、Sensend 实现对照（给改代码的人）

- 适配器文件：[lark.rs](file:///f:/fzz-Project/sensend/sensend/src-tauri/src/adapters/lark.rs)
- 调用流程：
  1. `get_tenant_token` — App ID+Secret 换 token，**进程内缓存 2 小时**（提前 5 分钟刷新）
  2. `resolve_document_id` — 识别 wiki 链接 → `get_node` 解析出 document_id
  3. `append_blocks` — 追加块，**每批最多 50 个 block**（`chunks(50)` 分批）
  4. `get_file_url` — 取文档链接用于跳转
- 错误语义（后端回传给前端）：
  - `飞书认证失败 (code=...)` → token 获取环节出错
  - `飞书 API 错误 (code=...): msg` → 业务接口报错（如 1770032 = 权限不足）
  - `HTTP 错误 (403)` → 接口级 403
- 前端错误翻译在 [usePlatform.ts](file:///f:/fzz-Project/sensend/sensend/src/composables/usePlatform.ts#L66-L72)：401→「Token 过期」、403/forbidden→「无权限访问目标」、429→「请求频繁」、网络→「网络失败」。**注意 403 类信息会被翻译成「无权限」，不代表 token 失效**。
