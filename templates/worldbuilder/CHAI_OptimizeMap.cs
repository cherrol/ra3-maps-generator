using System;
using System.IO;
using Dreamness.Ra3.Map.Facade.Core;

// CHAI 地图优化：自动地形混合 + 官方通行性 + 备份保存
// 用法：地编伴侣 -> 微程序 -> CHAI_OptimizeMap（可选传参 MapFilePath）
string? mapFilePath = null;

if (ArgumentDictionary != null
    && ArgumentDictionary.TryGetValue("MapFilePath", out var arg)
    && !string.IsNullOrWhiteSpace(arg))
{
    mapFilePath = arg;
}
else
{
    var dialog = Ra3MapUtils.ScriptViews.MapFileSelectorDialog.Create("选择要优化的地图 (.map)");
    dialog.ShowDialog();
    if (dialog.DialogResult == System.Windows.Forms.DialogResult.OK)
    {
        mapFilePath = dialog.Tag as string;
    }
}

if (string.IsNullOrWhiteSpace(mapFilePath))
{
    throw new Exception("未选择地图文件，程序终止。");
}

var ra3map = Ra3MapFacade.Open(mapFilePath);
var backup = ra3map.Backup();

// 1) 自动检测并写入地形混合（把硬边纹理交界变成过渡）
ra3map.AutoDetectBlendsEntireMap();

// 2) 用官方逻辑重算通行性
ra3map.UpdatePassabilityMap();

ra3map.Save();

Ra3MapUtils.ScriptViews.MsgDialog.Create(
    "CHAI 地图优化完成",
    $"地图: {Path.GetFileName(mapFilePath)}\n" +
    $"已执行: 自动地形混合 + 通行性重算\n" +
    $"备份: {backup}").ShowDialog();
