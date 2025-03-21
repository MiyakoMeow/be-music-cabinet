<template>
  <!-- 自定义标题栏容器 -->
  <div 
    class="custom-titlebar"
    :class="{ 'dark-mode': isDark }"
  >
    <!-- 可拖动区域 -->
    <div class="draggable-area">
      <span class="title">Be Music Cabinet</span>
    </div>

    <!-- 窗口控制按钮 -->
    <div class="window-controls">
      <button @click="minimizeWindow" title="最小化">
        <Icon icon="mdi:minimize" />
      </button>
      <button @click="closeWindow" class="close-btn" title="关闭">
        <Icon icon="mdi:close" />
      </button>
    </div>
  </div>
  <!-- 主容器添加overflow-hidden防止窗口滚动 -->
  <div 
    class="container"
    :class="{ 'dark-mode': isDark }"
    @dragover.prevent="onDragover"
    @drop.prevent="onDrop"
  >
    <!-- 上半部分：操作菜单 -->
    <div class="menu-bar">
      <div class="left-actions">
        <button @click="addDirectory">
          <Icon icon="mdi:folder-plus" /> 添加目录
        </button>
        <button @click="addArchive">
          <Icon icon="mdi:folder-zip" /> 添加压缩包
        </button>
        <button @click="deleteTrack">
          <Icon icon="mdi:trash-can" /> 删除
        </button>
      </div>

      <!-- 添加默认提示选项 -->
      <select v-model="selectedDirectory" class="directory-select">
        <option v-if="directories.length === 0" value="" disabled>
          请添加目录
        </option>
        <option 
          v-for="dir in directories" 
          :key="dir" 
          :value="dir"
        >
          {{ dir }}
        </option>
      </select>
    </div>

    <!-- 表格容器添加滚动 -->
    <div class="track-table">
      <table>
        <thead>
          <tr>
            <th v-for="col in columns" :key="col">{{ col }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="tracks.length === 0" class="placeholder">
            <td v-for="col in columns" :key="col">-</td>
          </tr>
          <tr v-else v-for="track in tracks" :key="track.id">
            <td>{{ track.id }}</td>
            <td>{{ track.title }}</td>
            <td>{{ track.artist }}</td>
            <td>{{ track.genre }}</td>
            <td>{{ track.sha256.slice(0, 4) }}</td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- 导入进度模态框 -->
    <div v-if="showImportModal" class="import-modal">
      <div class="modal-content">
        <h3>正在导入目录...</h3>
        <progress :value="importProgress" max="100"></progress>
        <p>{{ importProgress }}% 已完成</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { Icon } from '@iconify/vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from "@tauri-apps/api/window";

// 暗色模式状态
const isDark = ref(false)

// 初始化监听暗色模式
onMounted(async () => {
  const darkMode = matchMedia('(prefers-color-scheme: dark)').matches
  isDark.value = darkMode
})

// 窗口最小化
const minimizeWindow = async () => {
  try {
    console.info("窗口最小化")
    await getCurrentWindow().minimize()
  } catch (error) {
    console.error('最小化失败:', error)
  }
}

// 关闭窗口
const closeWindow = async () => {
  try {
    await getCurrentWindow().close()
  } catch (error) {
    console.error('关闭窗口失败:', error)
  }
}

// 处理标题栏拖动
// const handleDrag = (event: MouseEvent) => {
//   // getCurrentWindow().startDragging(event).catch(console.error)
//   getCurrentWindow().startDragging().catch(console.error)
// }

// 类型定义
interface Track {
  id: number
  title: string
  artist: string
  genre: string
  sha256: string
}

// 响应式数据
const tracks = ref<Track[]>([])
const directories = ref<string[]>([])
const selectedDirectory = ref<string>('') // 明确初始化空值
const showImportModal = ref(false)
const importProgress = ref(0)

// 表格列配置
const columns = ['ID', '标题', '艺术家', '曲风', 'SHA256']

// 初始化加载目录列表
onMounted(async () => {
  directories.value = await invoke('get_directories')
  if (directories.value.length > 0) {
    selectedDirectory.value = directories.value[0]
  }
})

// 拖放处理
const onDragover = (e: DragEvent) => {
  e.dataTransfer!.dropEffect = 'copy'
}

const onDrop = async (e: DragEvent) => {
  const files = Array.from(e.dataTransfer?.files || [])
  await invoke('handle_dropped_files', { files })
  await refreshTracks()
}

// 添加目录功能
const addDirectory = async () => {
  showImportModal.value = true
  try {
    await invoke('add_directory', {
      onProgress: (progress: number) => {
        importProgress.value = progress
      }
    })
  } finally {
    showImportModal.value = false
    await refreshTracks()
  }
}

// 刷新曲目列表
const refreshTracks = async () => {
  tracks.value = await invoke('get_tracks', {
    directory: selectedDirectory.value
  })
}

// 添加压缩包功能
const addArchive = async () => {
  showImportModal.value = true
  try {
    await invoke('add_directory', {
      onProgress: (progress: number) => {
        importProgress.value = progress
      }
    })
  } finally {
    showImportModal.value = false
    await refreshTracks()
  }
}

// 删除曲目功能
const deleteTrack = async () => {
  showImportModal.value = true
  try {
    await invoke('add_directory', {
      onProgress: (progress: number) => {
        importProgress.value = progress
      }
    })
  } finally {
    showImportModal.value = false
    await refreshTracks()
  }
}

</script>

<style scoped>
/* 自定义标题栏 */
.custom-titlebar {
  --titlebar-btn-hover: rgba(0, 0, 0, 0.1);
  --titlebar-close-hover: #e81123;
  height: 30px; /* 标题栏高度 */
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #f0f0f0;
  user-select: none;
  position: fixed; /* 固定定位 */
  top: 0;
  left: 0;
  right: 0;
  z-index: 9999; /* 确保标题栏在最上层 */
}

/* 暗色模式通过父容器限定 */
.custom-titlebar.dark-mode {
  --titlebar-btn-hover: rgba(255, 255, 255, 0.1);
  --titlebar-close-hover: #f1707a;
  background: #2d2d2d;
  color: white;
}

/* 所有子元素都通过父容器限定 */
.custom-titlebar .draggable-area {
  flex: 1;
  height: 100%;
  display: flex;
  align-items: center;
  padding-left: 12px;
  -webkit-app-region: drag;
  app-region: drag;
}

.custom-titlebar .title {
  font-size: 14px;
  opacity: 0.8;
}

.custom-titlebar .window-controls {
  display: flex;
  height: 100%;
  -webkit-app-region: no-drag;
  app-region: no-drag;
}

/* 按钮样式严格限定在标题栏内部 */
.custom-titlebar button {
  height: 100%;
  width: 46px;
  border: none;
  background: transparent;
  transition: background 0.2s;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* 悬停状态通过CSS变量控制 */
.custom-titlebar button:hover {
  background: var(--titlebar-btn-hover);
}

/* 关闭按钮特殊处理 */
.custom-titlebar .close-btn:hover {
  background: var(--titlebar-close-hover) !important;
  color: white;
}

th {
  position: sticky;
  top: 0;
  background: #252526;
  z-index: 1;
}

/* Sublime Text 暗色主题风格 */
.container {
  height: calc(100vh - 40px); /* 计算高度，减去标题栏的高度 */
  margin-top: 30px; /* 与标题栏的高度一致，确保容器在标题栏下方 */
  width: 100%; /* 确保宽度占满整个视口 */
  border: none;
  background: #1e1e1e;
  overflow: hidden; /* 新增 */
  color: #d4d4d4;
  font-family: -apple-system, BlinkMacSystemFont, sans-serif;
}

.menu-bar {
  padding: 8px;
  background: #2d2d2d;
  display: flex;
  justify-content: space-between;
  border-bottom: 1px solid #333;
}

button {
  background: #3c3c3c;
  color: #d4d4d4;
  border: 1px solid #333;
  padding: 6px 12px;
  margin-right: 8px;
  cursor: pointer;
  transition: all 0.2s;
}

button:hover {
  background: #454545;
}

th, td {
  padding: 8px 12px;
  text-align: left;
  border-bottom: 1px solid #333;
}

.placeholder td {
  color: #666;
  font-style: italic;
}

.import-modal {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: rgba(0,0,0,0.7);
  display: grid;
  place-items: center;
}

.modal-content {
  background: #252526;
  padding: 2rem;
  border-radius: 4px;
  text-align: center;
}

/* 修正类名并添加滚动样式 */
.track-table {
  height: calc(100vh - 30px); /* 计算可用高度 */
  overflow: auto; /* 垂直滚动 */
  position: relative;
}

/* 保持表格头固定 */
.track-table table {
  width: 100%;
  border-collapse: none;
}
</style>
