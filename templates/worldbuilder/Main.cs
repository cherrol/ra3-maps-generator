using System;
using System.Globalization;
using System.IO;
using MapCoreLib.Core.Asset;
using MapCoreLib.Util;

namespace MapCoreLib.Core.Scripts.ScriptFile
{
    // CHAI 专用：从 chai_objects.txt 导入对象并保留精确朝向
    // 文件格式（逗号分隔，与地编工作目录同名文件）：类型名,世界x,世界y,角度(度)
    public class Main : ScriptInterface
    {
        public void Apply(MapDataContext context)
        {
            var objectsList = context.getAsset<ObjectsList>(Ra3MapConst.ASSET_ObjectsList);
            int n = 0;
            foreach (var raw in File.ReadLines("chai_objects.txt"))
            {
                var line = raw.Trim();
                if (line.Length == 0 || line[0] == '#') continue;
                var t = line.Split(',');
                if (t.Length < 4) continue;
                double x = double.Parse(t[1], CultureInfo.InvariantCulture);
                double y = double.Parse(t[2], CultureInfo.InvariantCulture);
                double a = double.Parse(t[3], CultureInfo.InvariantCulture);
                objectsList.AddObject(context, t[0], new Vec3D(x, y), a);
                n++;
            }
        }
    }
}
