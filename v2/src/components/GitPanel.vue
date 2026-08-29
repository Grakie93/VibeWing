<script setup lang="ts">
import { ref, watch } from 'vue'
import { desktop } from '../services/desktop'
import type { ProjectView } from '../types'
const props=defineProps<{project:ProjectView}>();const files=ref<{path:string;status:string;staged:boolean;unstaged:boolean}[]>([]);const scope=ref('frontend');const message=ref('');const selected=ref<string[]>([]);const busy=ref(false)
async function refresh(){try{files.value=await desktop.gitStatus(props.project.id,scope.value)}catch(error){files.value=[];console.warn(error)}}
async function stage(){if(!selected.value.length)return;await desktop.gitStage(props.project.id,scope.value,selected.value);selected.value=[];await refresh()}
async function commit(){if(!message.value.trim())return;busy.value=true;try{await desktop.gitCommit(props.project.id,scope.value,message.value);message.value='';await refresh()}catch(error){alert(String(error))}finally{busy.value=false}}
async function push(){busy.value=true;try{await desktop.gitPush(props.project.id,scope.value);alert('已推送到远端')}catch(error){alert(String(error))}finally{busy.value=false}}
watch(scope,refresh,{immediate:true})
</script>
<template><details class="git-panel"><summary>Git</summary><div class="git-toolbar"><select v-model="scope"><option value="frontend">前端</option><option value="backend">后端</option></select><button @click="refresh">刷新</button></div><label v-for="file in files" :key="file.path" class="git-file"><input v-model="selected" type="checkbox" :value="file.path" :disabled="file.staged"/><span>{{file.status}}</span><code>{{file.path}}</code></label><button @click="stage">暂存所选</button><textarea v-model="message" placeholder="提交信息" rows="2"/><div class="actions"><button :disabled="busy" @click="commit">提交暂存</button><button :disabled="busy" class="primary" @click="push">推送远端</button></div></details></template>
