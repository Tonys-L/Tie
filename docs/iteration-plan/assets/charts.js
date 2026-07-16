(function() {
  var style = getComputedStyle(document.documentElement);
  var accent = style.getPropertyValue('--accent').trim();
  var accent2 = style.getPropertyValue('--accent2').trim();
  var ink = style.getPropertyValue('--ink').trim();
  var muted = style.getPropertyValue('--muted').trim();
  var rule = style.getPropertyValue('--rule').trim();
  var bg2 = style.getPropertyValue('--bg2').trim();
  var green = style.getPropertyValue('--green').trim();
  var yellow = style.getPropertyValue('--yellow').trim();
  var red = style.getPropertyValue('--red').trim();
  var orange = style.getPropertyValue('--orange').trim();

  // --- Chart 1: Iteration Timeline ---
  var timelineEl = document.getElementById('chart-timeline');
  if (timelineEl) {
    var chart1 = echarts.init(timelineEl, null, { renderer: 'svg' });
    chart1.setOption({
      animation: false,
      backgroundColor: 'transparent',
      title: {
        text: '4 个迭代的功能分布与工期预估',
        left: 'center',
        textStyle: { color: ink, fontSize: 14, fontWeight: 600 }
      },
      tooltip: {
        trigger: 'item',
        appendToBody: true,
        formatter: function(p) {
          return '<b>' + p.name + '</b><br/>' +
            '工期：' + p.value[3] + '<br/>' +
            '功能数：' + p.value[4] + ' 项';
        }
      },
      legend: {
        data: ['迭代一 v0.2.0', '迭代二 v0.3.0', '迭代三 v0.4.0', '迭代四 v1.0.0'],
        bottom: 0,
        textStyle: { color: muted, fontSize: 12 },
        itemWidth: 12,
        itemHeight: 12
      },
      grid: {
        left: '8%',
        right: '8%',
        top: '15%',
        bottom: '15%'
      },
      xAxis: {
        type: 'time',
        axisLine: { lineStyle: { color: rule } },
        axisLabel: { color: muted, fontSize: 11 },
        splitLine: { lineStyle: { color: rule, type: 'dashed' } }
      },
      yAxis: {
        type: 'category',
        data: ['', ''],
        axisLine: { show: false },
        axisLabel: { show: false },
        splitLine: { show: false }
      },
      series: [
        {
          name: '迭代一 v0.2.0',
          type: 'custom',
          renderItem: function(params, api) {
            var start = api.coord([api.value(0), 0.3]);
            var end = api.coord([api.value(1), 0.3]);
            var height = 24;
            return {
              type: 'rect',
              shape: {
                x: start[0],
                y: start[1] - height / 2,
                width: end[0] - start[0],
                height: height
              },
              style: {
                fill: accent,
                opacity: 0.85
              }
            };
          },
          encode: { x: [0, 1], y: 2 },
          data: [
            ['2026-07-16', '2026-08-05', 0.3, '2-3周', 3]
          ]
        },
        {
          name: '迭代二 v0.3.0',
          type: 'custom',
          renderItem: function(params, api) {
            var start = api.coord([api.value(0), 0.5]);
            var end = api.coord([api.value(1), 0.5]);
            var height = 24;
            return {
              type: 'rect',
              shape: {
                x: start[0],
                y: start[1] - height / 2,
                width: end[0] - start[0],
                height: height
              },
              style: {
                fill: accent2,
                opacity: 0.85
              }
            };
          },
          encode: { x: [0, 1], y: 2 },
          data: [
            ['2026-08-06', '2026-08-19', 0.5, '1-2周', 2]
          ]
        },
        {
          name: '迭代三 v0.4.0',
          type: 'custom',
          renderItem: function(params, api) {
            var start = api.coord([api.value(0), 0.7]);
            var end = api.coord([api.value(1), 0.7]);
            var height = 24;
            return {
              type: 'rect',
              shape: {
                x: start[0],
                y: start[1] - height / 2,
                width: end[0] - start[0],
                height: height
              },
              style: {
                fill: yellow,
                opacity: 0.85
              }
            };
          },
          encode: { x: [0, 1], y: 2 },
          data: [
            ['2026-08-20', '2026-09-02', 0.7, '2周', 2]
          ]
        },
        {
          name: '迭代四 v1.0.0',
          type: 'custom',
          renderItem: function(params, api) {
            var start = api.coord([api.value(0), 0.9]);
            var end = api.coord([api.value(1), 0.9]);
            var height = 24;
            return {
              type: 'rect',
              shape: {
                x: start[0],
                y: start[1] - height / 2,
                width: end[0] - start[0],
                height: height
              },
              style: {
                fill: orange,
                opacity: 0.85
              }
            };
          },
          encode: { x: [0, 1], y: 2 },
          data: [
            ['2026-09-03', '2026-10-07', 0.9, '3-4周', 3]
          ]
        }
      ]
    });
    window.addEventListener('resize', function() { chart1.resize(); });
  }

  // --- Chart 2: Priority Matrix ---
  var priorityEl = document.getElementById('chart-priority');
  if (priorityEl) {
    var chart2 = echarts.init(priorityEl, null, { renderer: 'svg' });
    chart2.setOption({
      animation: false,
      backgroundColor: 'transparent',
      title: {
        text: '功能优先级矩阵',
        left: 'center',
        textStyle: { color: ink, fontSize: 14, fontWeight: 600 }
      },
      tooltip: {
        trigger: 'item',
        appendToBody: true,
        formatter: function(p) {
          return '<b>' + p.data[3] + '</b><br/>' +
            '用户影响度：' + p.data[0] + '/10<br/>' +
            '开发工作量：' + p.data[1] + ' 天<br/>' +
            '优先级：' + p.data[4];
        }
      },
      legend: {
        data: ['P0 高优先级', 'P1 较高', 'P2 中优先级', 'P3 低优先级', 'P4 长期'],
        bottom: 0,
        textStyle: { color: muted, fontSize: 11 },
        itemWidth: 10,
        itemHeight: 10
      },
      grid: {
        left: '10%',
        right: '8%',
        top: '12%',
        bottom: '15%'
      },
      xAxis: {
        name: '开发工作量（天）',
        nameLocation: 'middle',
        nameGap: 30,
        nameTextStyle: { color: muted, fontSize: 12 },
        type: 'value',
        min: 0,
        max: 30,
        axisLine: { lineStyle: { color: rule } },
        axisLabel: { color: muted, fontSize: 11 },
        splitLine: { lineStyle: { color: rule, type: 'dashed' } }
      },
      yAxis: {
        name: '用户影响度',
        nameLocation: 'middle',
        nameGap: 40,
        nameTextStyle: { color: muted, fontSize: 12 },
        type: 'value',
        min: 0,
        max: 10,
        axisLine: { lineStyle: { color: rule } },
        axisLabel: { color: muted, fontSize: 11 },
        splitLine: { lineStyle: { color: rule, type: 'dashed' } }
      },
      series: [
        {
          name: 'P0 高优先级',
          type: 'scatter',
          symbolSize: function(data) { return 16; },
          itemStyle: { color: red, opacity: 0.85 },
          data: [
            [3, 9.5, 16, '全局搜索', 'P0'],
            [4, 9.0, 16, '分类标签', 'P0'],
            [2.5, 8.5, 16, '待办清单', 'P0']
          ],
          label: {
            show: true,
            position: 'top',
            formatter: function(p) { return p.data[3]; },
            color: ink,
            fontSize: 11,
            fontWeight: 600
          }
        },
        {
          name: 'P1 较高',
          type: 'scatter',
          symbolSize: 14,
          itemStyle: { color: orange, opacity: 0.85 },
          data: [
            [3, 7.5, 14, '便签列表', 'P1']
          ],
          label: {
            show: true,
            position: 'top',
            formatter: function(p) { return p.data[3]; },
            color: ink,
            fontSize: 11,
            fontWeight: 600
          }
        },
        {
          name: 'P2 中优先级',
          type: 'scatter',
          symbolSize: 12,
          itemStyle: { color: yellow, opacity: 0.85 },
          data: [
            [2, 6.0, 12, '数据导出', 'P2'],
            [4, 7.0, 12, '日历视图', 'P2']
          ],
          label: {
            show: true,
            position: 'top',
            formatter: function(p) { return p.data[3]; },
            color: ink,
            fontSize: 11,
            fontWeight: 600
          }
        },
        {
          name: 'P3 低优先级',
          type: 'scatter',
          symbolSize: 10,
          itemStyle: { color: accent2, opacity: 0.85 },
          data: [
            [3.5, 4.5, 10, '农历提醒', 'P3'],
            [3, 5.0, 10, 'Webhook通知', 'P3'],
            [3, 4.0, 10, '便签模板', 'P3']
          ],
          label: {
            show: true,
            position: 'top',
            formatter: function(p) { return p.data[3]; },
            color: muted,
            fontSize: 10
          }
        },
        {
          name: 'P4 长期',
          type: 'scatter',
          symbolSize: 10,
          itemStyle: { color: accent, opacity: 0.6 },
          data: [
            [4.5, 6.5, 10, 'Git版本可视化', 'P4']
          ],
          label: {
            show: true,
            position: 'top',
            formatter: function(p) { return p.data[3]; },
            color: muted,
            fontSize: 10
          }
        }
      ]
    });
    window.addEventListener('resize', function() { chart2.resize(); });
  }
})();
