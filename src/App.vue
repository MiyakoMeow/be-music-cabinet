<template>
  <!-- 主容器，处理拖放事件 -->
  <div 
    class="container"
    @dragover.prevent="onDragover"
    @drop.prevent="onDrop"
  >
    <!-- 上半部分：操作菜单 -->
    <div class="menu-bar">
      <div class="left-actions">
        <!-- 操作按钮 -->
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
      
      <!-- 目录选择 -->
      <select v-model="selectedDirectory" class="directory-select">
        <option v-for="dir in directories" :key="dir" :value="dir">
          {{ dir }}
        </option>
      </select>
    </div>

    <!-- 下半部分：数据表格 -->
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
import { invoke } from '@tauri-apps/api/core'
import { Icon } from '@iconify/vue'

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
const selectedDirectory = ref('')
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

<style>
/* Sublime Text 暗色主题风格 */
.container {
  height: 100vh;
  background: #1e1e1e;
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

track-table {
  overflow: auto;
  height: calc(100vh - 60px);
}

table {
  width: 100%;
  border-collapse: collapse;
}

th, td {
  padding: 8px 12px;
  text-align: left;
  border-bottom: 1px solid #333;
}

th {
  background: #252526;
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
</style>
