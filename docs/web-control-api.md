# Web 控制接口文档

本文档说明局域网 Web 控制台调用 PC 控制能力的两种方式：

- WebSocket：主通道，用于状态同步、心跳和实时执行指令。
- HTTP API：兜底通道，WebSocket 不可用时仍可执行指令，也用于定时任务。

所有 Web 控制台接口只允许局域网或本机地址访问。服务端会通过请求来源 IP 判断，不在局域网内时返回：

```json
{
  "success": false,
  "msg": "Web console is only available on the local network"
}
```

## 基础地址

Web 控制台由 PC 端 Tauri/Axum 服务托管。下面示例用 `http://PC_IP:PORT` 表示实际访问地址。

```text
http://PC_IP:PORT/web
ws://PC_IP:PORT/web/ws
```

如果页面使用 HTTPS，对应 WebSocket 协议为 `wss://PC_IP:PORT/web/ws`。

## 指令格式

后端指令使用 `feature` 字段区分功能。除 `volume` 外，其它指令不需要额外参数。

```json
{
  "feature": "shutdown"
}
```

设置音量需要 `level`，范围是 `0` 到 `100`。

```json
{
  "feature": "volume",
  "level": 50
}
```

可选的 Web 客户端信息字段如下。服务端用于记录来源和桌面端提示，不是鉴权字段。

```json
{
  "client_info": {
    "clientId": "browser-client-id",
    "deviceName": "Windows",
    "deviceModel": "Surface",
    "platform": "Windows",
    "browser": "Edge",
    "userAgent": "Mozilla/5.0 ..."
  }
}
```

## 功能指令列表

| 指令 | 功能 | 参数 | 风险 |
| --- | --- | --- | --- |
| `shutdown` | 关闭 PC | 无 | 高 |
| `restart` | 重启 PC | 无 | 高 |
| `test_notification` | 触发测试提示 | 无 | 低 |
| `error_test` | 触发错误测试 | 无 | 低 |
| `volume` | 设置系统音量 | `level`: 0-100 | 低 |
| `apple_music_open` | 打开 Apple Music | 无 | 低 |
| `apple_music_previous` | Apple Music 上一曲 | 无 | 低 |
| `apple_music_play_pause` | Apple Music 播放/暂停切换 | 无 | 低 |
| `apple_music_next` | Apple Music 下一曲 | 无 | 低 |

注意：状态接口返回的 `featureKey` 是前端控件标识，大多数情况下和 `feature` 相同；但媒体播放器容器可能返回 `apple_music_player`，实际点击动作仍要发送表格里的 `apple_music_previous`、`apple_music_play_pause`、`apple_music_next`。

## WebSocket 接口

### 连接

```text
GET /web/ws
```

浏览器示例：

```js
const socket = new WebSocket(`ws://${location.host}/web/ws`);
```

连接建立后，服务端会主动推送一次 `state_sync`。

### 心跳

客户端发送：

```json
{
  "type": "heartbeat"
}
```

服务端返回：

```json
{
  "type": "pong"
}
```

当前 Web 前端每 15 秒发送一次心跳。

### 请求状态同步

客户端发送：

```json
{
  "type": "request_state_sync",
  "client_info": {
    "clientId": "browser-client-id",
    "deviceName": "Windows",
    "platform": "Windows",
    "browser": "Edge"
  }
}
```

服务端返回 `state_sync`，结构见“状态同步事件”。

### 执行功能指令

客户端发送：

```json
{
  "type": "execute_feature",
  "request_id": "ab2f13aa-8fd6-4e6a-9b88-9a2e5c7ce301",
  "client_info": {
    "clientId": "browser-client-id",
    "deviceName": "Windows",
    "platform": "Windows",
    "browser": "Edge"
  },
  "feature": "volume",
  "level": 50
}
```

`request_id` 由客户端生成，用于把服务端返回的执行结果和本次请求对应起来。

服务端返回：

```json
{
  "type": "feature_result",
  "request_id": "ab2f13aa-8fd6-4e6a-9b88-9a2e5c7ce301",
  "success": true,
  "msg": "音量已设置为 50%",
  "result": {
    "featureKey": "volume",
    "message": "音量已设置为 50%",
    "volumeLevel": 50,
    "appleMusicRunning": null,
    "appleMusicPlaybackState": null,
    "appleMusicTrack": null
  }
}
```

执行失败时：

```json
{
  "type": "feature_result",
  "request_id": "ab2f13aa-8fd6-4e6a-9b88-9a2e5c7ce301",
  "success": false,
  "msg": "错误信息",
  "result": null
}
```

### 主动断开

客户端发送：

```json
{
  "type": "disconnect"
}
```

### 状态同步事件

服务端在连接建立、客户端请求同步、功能执行后或任务变化后推送：

```json
{
  "type": "state_sync",
  "groups": [],
  "snapshot": {
    "volumeLevel": 50,
    "appleMusicRunning": true,
    "appleMusicPlaybackState": "playing",
    "appleMusicTrack": {
      "title": "Song",
      "artist": "Artist",
      "album": "Album",
      "albumArtist": "Album Artist",
      "artworkDataUrl": "data:image/png;base64,...",
      "positionMs": 10000,
      "durationMs": 180000
    }
  },
  "tasks": [],
  "history": []
}
```

## HTTP API

HTTP API 使用 JSON 请求体和 JSON 响应。除 `GET /web/api/state` 外，请求头建议带：

```text
Content-Type: application/json
```

### 获取状态

```text
GET /web/api/state
```

成功响应：

```json
{
  "success": true,
  "msg": "OK",
  "groups": [],
  "snapshot": {
    "volumeLevel": 50,
    "appleMusicRunning": false,
    "appleMusicPlaybackState": "unavailable",
    "appleMusicTrack": null
  },
  "tasks": [],
  "history": []
}
```

### 执行功能指令

```text
POST /web/api/features/execute
```

请求示例：

```json
{
  "client_info": {
    "clientId": "browser-client-id",
    "deviceName": "Windows",
    "platform": "Windows",
    "browser": "Edge"
  },
  "feature": "apple_music_play_pause"
}
```

`curl` 示例：

```bash
curl -X POST "http://PC_IP:PORT/web/api/features/execute" \
  -H "Content-Type: application/json" \
  -d "{\"feature\":\"volume\",\"level\":50}"
```

成功响应：

```json
{
  "success": true,
  "msg": "执行成功信息",
  "result": {
    "featureKey": "volume",
    "message": "执行成功信息",
    "volumeLevel": 50,
    "appleMusicRunning": null,
    "appleMusicPlaybackState": null,
    "appleMusicTrack": null
  }
}
```

失败响应：

```json
{
  "success": false,
  "msg": "错误信息",
  "result": null
}
```

### 创建定时任务

```text
POST /web/api/tasks/create
```

`execute_at_ms` 是 Unix 毫秒时间戳，必须晚于当前时间。

请求示例：

```json
{
  "client_info": {
    "clientId": "browser-client-id",
    "deviceName": "Windows",
    "platform": "Windows",
    "browser": "Edge"
  },
  "execute_at_ms": 1778035200000,
  "feature": "shutdown"
}
```

设置定时音量：

```json
{
  "execute_at_ms": 1778035200000,
  "feature": "volume",
  "level": 30
}
```

成功响应：

```json
{
  "success": true,
  "msg": "定时任务已创建",
  "task": {
    "taskId": "5b9b94a0-7b0b-4b55-bec2-e6e68f70f7e8",
    "title": "定时关机",
    "createdAtMs": 1778031600000,
    "executeAtMs": 1778035200000,
    "origin": {
      "kind": "web",
      "clientId": "browser-client-id",
      "clientName": "Windows Edge · Web 控制台 192.168.1.10"
    },
    "feature": "shutdown"
  }
}
```

### 取消定时任务

```text
POST /web/api/tasks/cancel
```

请求示例：

```json
{
  "client_info": {
    "clientId": "browser-client-id"
  },
  "task_id": "5b9b94a0-7b0b-4b55-bec2-e6e68f70f7e8"
}
```

成功响应：

```json
{
  "success": true,
  "msg": "定时任务已停止",
  "task": {
    "taskId": "5b9b94a0-7b0b-4b55-bec2-e6e68f70f7e8",
    "title": "定时关机",
    "createdAtMs": 1778031600000,
    "executeAtMs": 1778035200000,
    "origin": {
      "kind": "web",
      "clientId": "browser-client-id",
      "clientName": "Windows Edge · Web 控制台 192.168.1.10"
    },
    "feature": "shutdown"
  }
}
```

未找到可取消任务时：

```json
{
  "success": false,
  "msg": "未找到可停止的定时任务",
  "task": null
}
```

## 响应字段说明

### FeatureExecutionResult

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `featureKey` | string | 已执行的功能键 |
| `message` | string | 执行结果文案 |
| `volumeLevel` | number 或 null | 音量指令执行后的音量 |
| `appleMusicRunning` | boolean 或 null | Apple Music 是否运行 |
| `appleMusicPlaybackState` | string 或 null | Apple Music 播放状态 |
| `appleMusicTrack` | object 或 null | Apple Music 当前曲目信息 |

### ScheduledTask

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `taskId` | string | 任务 ID |
| `title` | string | 任务标题 |
| `createdAtMs` | number | 创建时间，Unix 毫秒时间戳 |
| `executeAtMs` | number | 计划执行时间，Unix 毫秒时间戳 |
| `origin` | object | 来源信息 |
| `feature` | string | 指令名 |
| `level` | number | 仅 `volume` 任务存在 |

### TaskOrigin

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `kind` | `pc` / `mobile` / `web` | 来源类型 |
| `clientId` | string 或 null | 客户端 ID |
| `clientName` | string | 客户端显示名 |

## 当前前端调用策略

Web 前端当前策略是：

1. 页面加载时先通过 `GET /web/api/state` 获取初始状态。
2. 同时连接 `/web/ws`，连接成功后使用 WebSocket 接收状态同步。
3. 执行功能时，如果 WebSocket 已连接，发送 `execute_feature`。
4. 如果 WebSocket 不可用，降级调用 `POST /web/api/features/execute`。
5. 创建和取消定时任务始终走 HTTP API。

## 资源同步接口

本节记录 PC 和 Web 设备之间同步文本、图片和任意文件的接口。资源同步历史由 PC 维护，不依赖浏览器 `localStorage`。

### 存储策略

每次发送算一条消息。文本、单个文件、一组文件、文本加文件都保存为同一条消息。PC 端以单条消息为单位落盘：

```text
{app_data_dir}/clipboard-sync/messages/{createdAtMs}-{messageId}/
  message.json
  {attachmentId}_{fileName}
```

`message.json` 保存消息元信息，附件文件保存在同一目录。PC 管理页支持按单条消息删除，也支持清空全部同步记录。当前最多保留 500 条，超过后会清理最旧消息。

### WebSocket 同步事件

资源同步历史变化后，服务端会向 `/web/ws` 连接推送：

```json
{
  "type": "clipboard_sync",
  "messages": [
    {
      "messageId": "1778031600000-5b9b94a0-7b0b-4b55-bec2-e6e68f70f7e8",
      "createdAtMs": 1778031600000,
      "source": {
        "kind": "web",
        "clientId": "browser-client-id",
        "deviceName": "iPhone",
        "deviceModel": "iPhone",
        "platform": "iOS",
        "browser": "Safari",
        "ip": "192.168.1.10"
      },
      "text": "hello",
      "attachments": []
    }
  ]
}
```

`state_sync` 事件和 `GET /web/api/state` 响应中也会包含 `syncMessages` 字段，结构同上。

### 获取同步历史

```text
GET /web/api/sync/history
```

成功响应：

```json
{
  "success": true,
  "msg": "OK",
  "message": null,
  "messages": []
}
```

### 创建同步消息

```text
POST /web/api/sync/messages
Content-Type: multipart/form-data
```

表单字段：

| 字段 | 类型 | 必填 | 说明 |
| --- | --- | --- | --- |
| `source_kind` | `web` / `pc` | 否 | 来源类型，默认 `web`。Web 来源会同步写入 PC 剪切板。 |
| `client_info` | JSON string | 否 | Web 设备信息，用于记录设备名、机型、平台、浏览器和 IP。 |
| `text` | string | 否 | 要同步的文本。 |
| `files` / `files[]` | file | 否 | 附件，可多选，支持任意文件。 |

`text` 和 `files` 至少需要提供一个。

浏览器示例：

```js
const form = new FormData();
form.set("source_kind", "web");
form.set("client_info", JSON.stringify({
  clientId: "browser-client-id",
  deviceName: "iPhone",
  deviceModel: "iPhone",
  platform: "iOS",
  browser: "Safari",
}));
form.set("text", "hello");
files.forEach((file) => form.append("files", file, file.name));

await fetch("/web/api/sync/messages", {
  method: "POST",
  body: form,
});
```

成功响应：

```json
{
  "success": true,
  "msg": "已同步",
  "message": {
    "messageId": "1778031600000-5b9b94a0-7b0b-4b55-bec2-e6e68f70f7e8",
    "createdAtMs": 1778031600000,
    "source": {
      "kind": "web",
      "clientId": "browser-client-id",
      "deviceName": "iPhone",
      "deviceModel": "iPhone",
      "platform": "iOS",
      "browser": "Safari",
      "ip": "192.168.1.10"
    },
    "text": "hello",
    "attachments": [
      {
        "attachmentId": "8fd6a6d0-9f10-4a42-8ffc-111111111111",
        "fileName": "photo.png",
        "storedName": "8fd6a6d0-9f10-4a42-8ffc-111111111111_photo.png",
        "mimeType": "image/png",
        "sizeBytes": 102400
      }
    ]
  },
  "messages": []
}
```

Web 来源的剪切板规则：

- 只有文本：写入 PC 文本剪切板。
- 有附件：附件保存到 PC 磁盘，并把保存后的文件路径列表写入 PC 文件剪切板，支持多图和任意文件。
- 文本加附件：文本保存到历史，附件写入文件剪切板。

PC 来源不会再次写入 PC 剪切板，只创建历史并广播给 Web。

### 下载同步附件

```text
GET /web/api/sync/files/:message_id/:attachment_id
```

返回附件二进制内容。响应会尽量带上原始 `Content-Type`，并使用 `Content-Disposition: attachment` 便于浏览器保存。

### 字段说明

#### ClipboardSyncMessage

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `messageId` | string | 消息 ID，也是 PC 磁盘消息目录名的一部分 |
| `createdAtMs` | number | 创建时间，Unix 毫秒时间戳 |
| `source` | object | 来源信息 |
| `text` | string 或 null | 同步文本 |
| `attachments` | array | 附件列表 |

#### ClipboardSyncSource

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `kind` | `pc` / `web` | 来源类型 |
| `clientId` | string 或 null | 客户端 ID |
| `deviceName` | string 或 null | 设备名 |
| `deviceModel` | string 或 null | 设备机型，浏览器可提供时记录 |
| `platform` | string 或 null | 操作系统或平台 |
| `browser` | string 或 null | 浏览器名称 |
| `ip` | string 或 null | 请求来源 IP |

#### ClipboardSyncAttachment

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `attachmentId` | string | 附件 ID |
| `fileName` | string | 原始文件名，经过安全清理 |
| `storedName` | string | PC 磁盘中的保存文件名 |
| `mimeType` | string 或 null | 上传时的 MIME 类型 |
| `sizeBytes` | number | 文件大小 |
