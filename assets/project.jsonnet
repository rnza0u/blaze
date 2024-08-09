{
    targets: {
        source: {
            cache: {
                invalidateWhen: {
                    inputChanges: [
                        'favicons/**',
                        'logos/**'
                    ]
                }
            }
        }
    }
}